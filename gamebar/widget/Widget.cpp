// Local rendering companion for the tracker. No game process access or input injection.
#define NOMINMAX
#include <collection.h>
#include <ppltasks.h>
#include <windows.h>
#include <cmath>
#include <string>
#include <algorithm>

using namespace Platform;
using namespace Windows::ApplicationModel::Activation;
using namespace Windows::Data::Json;
using namespace Windows::Foundation;
using namespace Windows::Storage;
using namespace Windows::UI;
using namespace Windows::UI::Core;
using namespace Windows::UI::Xaml;
using namespace Windows::UI::Xaml::Controls;
using namespace Windows::UI::Xaml::Media;
using namespace Microsoft::Gaming::XboxGameBar;

namespace {
LONGLONG ticks() { LARGE_INTEGER n; QueryPerformanceCounter(&n); return n.QuadPart; }
String^ text(const std::wstring& value) { return ref new String(value.c_str()); }
std::wstring local(const wchar_t* name) { return std::wstring(ApplicationData::Current->LocalFolder->Path->Data())+L"\\"+name; }
std::string read_small(const std::wstring& path) {
    CREATEFILE2_EXTENDED_PARAMETERS options{}; options.dwSize=sizeof(options);
    auto h=CreateFile2(path.c_str(),GENERIC_READ,FILE_SHARE_READ|FILE_SHARE_WRITE|FILE_SHARE_DELETE,OPEN_EXISTING,&options);
    if(h==INVALID_HANDLE_VALUE)return {};
    LARGE_INTEGER size{};std::string value;
    if(GetFileSizeEx(h,&size)&&size.QuadPart>0&&size.QuadPart<=8192){value.resize(static_cast<size_t>(size.QuadPart));DWORD n=0;if(!ReadFile(h,&value[0],static_cast<DWORD>(value.size()),&n,nullptr))value.clear();else value.resize(n);}
    CloseHandle(h);return value;
}
String^ utf8(const std::string& value) {
    if(value.empty())return L"";
    auto size=MultiByteToWideChar(CP_UTF8,MB_ERR_INVALID_CHARS,value.data(),static_cast<int>(value.size()),nullptr,0);
    if(size<=0)return L"";std::wstring wide(size,L'\0');MultiByteToWideChar(CP_UTF8,0,value.data(),static_cast<int>(value.size()),&wide[0],size);return text(wide);
}
void write_status(bool pinned,bool clickthrough,bool visible,double opacity,int render_error,bool fresh,bool showing,double remaining,int display_mode,float width,float height) {
    auto j=ref new JsonObject();j->Insert(L"schema",JsonValue::CreateNumberValue(1));
    j->Insert(L"pinned",JsonValue::CreateBooleanValue(pinned));j->Insert(L"click_through",JsonValue::CreateBooleanValue(clickthrough));
    j->Insert(L"visible",JsonValue::CreateBooleanValue(visible));j->Insert(L"heartbeat_ticks",JsonValue::CreateNumberValue(static_cast<double>(ticks())));
    j->Insert(L"requested_opacity",JsonValue::CreateNumberValue(opacity));
    j->Insert(L"render_error",JsonValue::CreateNumberValue(render_error));
    j->Insert(L"bridge_fresh",JsonValue::CreateBooleanValue(fresh));
    j->Insert(L"timer_visible",JsonValue::CreateBooleanValue(showing));
    j->Insert(L"remaining",JsonValue::CreateNumberValue(std::max(0.0,remaining)));
    j->Insert(L"display_mode",JsonValue::CreateNumberValue(display_mode));
    j->Insert(L"content_width",JsonValue::CreateNumberValue(width));
    j->Insert(L"content_height",JsonValue::CreateNumberValue(height));
    auto data=j->Stringify();auto bytes=Windows::Security::Cryptography::CryptographicBuffer::ConvertStringToBinary(data,Windows::Security::Cryptography::BinaryStringEncoding::Utf8);
    // This callback only writes a tiny heartbeat to our own package LocalState.
    concurrency::create_task(ApplicationData::Current->LocalFolder->CreateFileAsync(L"widget-status.json",CreationCollisionOption::ReplaceExisting))
        .then([bytes](StorageFile^ file){return FileIO::WriteBufferAsync(file,bytes);}).then([](concurrency::task<void> t){try{t.get();}catch(Exception^){} });
}
}

namespace Katsu { namespace SpikeWidget {
public ref class App sealed : Application {
    XboxGameBarWidget^ widget;
    Frame^ frame;
    Canvas^ canvas;
    Border^ badge;
    TextBlock^ seconds;
    DispatcherTimer^ timer;
    JsonObject^ state;
    LONGLONG last_read=0,last_heartbeat=0,frequency=1;
    std::wstring previous;
    bool idle_minimize_requested=false;
    LONGLONG last_minimize=0;
    LONGLONG last_resize=0;
    int render_error=0;
    float previous_width=0,previous_height=0;
public:
    App() { LARGE_INTEGER f;QueryPerformanceFrequency(&f);frequency=f.QuadPart; }
protected:
    virtual void OnLaunched(LaunchActivatedEventArgs^ args) override { CreateView(nullptr); }
    virtual void OnActivated(IActivatedEventArgs^ args) override {
        if(args->Kind!=ActivationKind::Protocol)return;
        auto protocol=dynamic_cast<IProtocolActivatedEventArgs^>(args);
        if(protocol==nullptr||protocol->Uri->SchemeName!=L"ms-gamebarwidget")return;
        auto activated=dynamic_cast<XboxGameBarWidgetActivatedEventArgs^>(args);
        if(activated!=nullptr&&activated->IsLaunchActivation)CreateView(activated);
    }
private:
    void CreateView(XboxGameBarWidgetActivatedEventArgs^ activated) {
        canvas=ref new Canvas();canvas->Background=nullptr;canvas->IsHitTestVisible=false;
        frame=ref new Frame();frame->Background=nullptr;frame->IsHitTestVisible=false;frame->Content=canvas;
        Window::Current->Content=frame;
        seconds=ref new TextBlock();seconds->FontFamily=ref new FontFamily(L"Segoe UI");seconds->FontSize=30;
        seconds->FontWeight=Windows::UI::Text::FontWeights::SemiBold;seconds->Text=L"";seconds->TextAlignment=TextAlignment::Center;
        badge=ref new Border();badge->Child=seconds;badge->Padding=Thickness(6,2,6,2);badge->CornerRadius=CornerRadius(6);
        badge->IsHitTestVisible=false;badge->Visibility=Visibility::Collapsed;canvas->Children->Append(badge);
        if(activated!=nullptr){
            widget=ref new XboxGameBarWidget(activated,Window::Current->CoreWindow,frame);
            widget->PinningSupported=true;widget->HorizontalResizeSupported=false;widget->VerticalResizeSupported=false;
            widget->MinWindowSize=Size(40,24);widget->MaxWindowSize=Size(300,160);
        }
        timer=ref new DispatcherTimer();TimeSpan interval;interval.Duration=1000000;timer->Interval=interval;
        timer->Tick+=ref new Windows::Foundation::EventHandler<Object^>(this,&App::Tick);timer->Start();
        Window::Current->Activate();
    }
    void Resize(float width,float height) {
        if(widget==nullptr)return;
        if(std::abs(width-previous_width)<1&&std::abs(height-previous_height)<1)return;
        const auto now=ticks();if(now-last_resize<frequency/2)return;
        last_resize=now;previous_width=width;previous_height=height;
        concurrency::create_task(widget->TryResizeWindowAsync(Size(width,height))).then([this](concurrency::task<bool> t){
            try{if(!t.get()){previous_width=0;previous_height=0;}}catch(Exception^ error){render_error=error->HResult;previous_width=0;previous_height=0;}
        },concurrency::task_continuation_context::use_current());
    }
    void Tick(Object^,Object^) {
        try {
            const auto now=ticks();
            if(now-last_read>frequency/5){
                last_read=now;auto raw=read_small(local(L"tracker-state.json"));
                if(!raw.empty()){auto value=utf8(raw);if(previous!=value->Data()){state=JsonObject::Parse(value);previous=value->Data();}}
            }
            bool pinned=widget!=nullptr&&widget->Pinned;
            bool through=widget!=nullptr&&widget->ClickThroughEnabled;
            bool foreground=widget!=nullptr&&widget->GameBarDisplayMode==XboxGameBarDisplayMode::Foreground;
            bool fresh=state!=nullptr&&state->GetNamedNumber(L"schema",0)==1&&state->GetNamedNumber(L"heartbeat_ticks",0)<=now&&now-state->GetNamedNumber(L"heartbeat_ticks",0)<frequency*3;
            bool active=fresh&&state->GetNamedBoolean(L"visible",false);
            const auto real_remaining=active?(state->GetNamedNumber(L"expires_ticks",0)-now)/frequency:0;
            const bool real_timer=active&&real_remaining>0&&real_remaining<=45&&pinned;
            const bool preview=foreground&&!real_timer;
            const bool show=real_timer||preview;
            const double remaining=real_timer?real_remaining:preview?45.0:0.0;
            if(now-last_heartbeat>frequency){last_heartbeat=now;auto bounds=Window::Current->Bounds;write_status(pinned,through,widget!=nullptr&&widget->Visible,widget!=nullptr?widget->RequestedOpacity:1.0,render_error,fresh,real_timer,real_remaining,widget!=nullptr?static_cast<int>(widget->GameBarDisplayMode):-1,bounds.Width,bounds.Height);}
            badge->Visibility=show?Visibility::Visible:Visibility::Collapsed;
            if(show)idle_minimize_requested=false;
            if(!show){
                // No transparent input rectangle may remain after a countdown or
                // tracker shutdown. The desktop bridge restores the widget on plant.
                const bool configuring=fresh&&state->GetNamedBoolean(L"configuring",false);
                if(!foreground&&!configuring&&widget!=nullptr&&widget->Visible&&(!idle_minimize_requested||now-last_minimize>=frequency)){
                    idle_minimize_requested=true;last_minimize=now;
                    concurrency::create_task(widget->MinimizeAsync()).then([this](concurrency::task<void> t){try{t.get();}catch(Exception^ error){render_error=error->HResult;}},concurrency::task_continuation_context::use_current());
                }
                return;
            }
            auto font=std::max(14.0,std::min(84.0,state!=nullptr?state->GetNamedNumber(L"font_size",30):30.0));
            seconds->FontSize=font;seconds->LineHeight=font*1.2;seconds->LineStackingStrategy=LineStackingStrategy::BlockLineHeight;seconds->MinWidth=font*2.5;
            wchar_t buffer[32];swprintf_s(buffer,L"%.1f",remaining);seconds->Text=ref new String(buffer);
            auto color=remaining>7.5?Color{255,131,230,183}:remaining>4.0?Color{255,255,218,106}:Color{255,255,107,113};
            seconds->Foreground=ref new SolidColorBrush(color);
            auto opacity=std::max(0.0,std::min(1.0,state!=nullptr?state->GetNamedNumber(L"background_opacity",.78):.78));
            badge->Background=ref new SolidColorBrush(Color{static_cast<unsigned char>(opacity*255),8,14,18});
            canvas->Opacity=widget!=nullptr?widget->RequestedOpacity:1.0;
            badge->Measure(Size(10000,10000));auto desired=badge->DesiredSize;
            // Game Bar owns position. Never center or expand to a whole display:
            // resizing only to the visible number preserves the user's drag position.
            Resize(std::max(40.0f,static_cast<float>(std::ceil(desired.Width))),std::max(24.0f,static_cast<float>(std::ceil(desired.Height))));
            Canvas::SetLeft(badge,0);Canvas::SetTop(badge,0);
            render_error=0;
        } catch(Exception^ error) {render_error=error->HResult;if(badge!=nullptr)badge->Visibility=Visibility::Collapsed;}
    }
};
}}

[MTAThread] int main(Array<String^>^) {
    Application::Start(ref new ApplicationInitializationCallback([](ApplicationInitializationCallbackParams^){ref new Katsu::SpikeWidget::App();}));
}
