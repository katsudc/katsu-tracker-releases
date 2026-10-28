# katsu's Tracker

[English](#english) · [Türkçe](#türkçe)

## Türkçe

VALORANT için katsu tarafından geliştirilen Windows masaüstü arayüzü.

- Canlı maçta oyuncu adları, ajanlar ve rütbeler; mevcut kaplama görünümü korunur.
- Haritaya göre ajan seçimi ve otomatik kilitleme tercihleri.
- Spike simgesini izleyen, oyun monitöründe görünen sade sayaç.
- Beş renk teması, Türkçe/İngilizce arayüz ve otomatik kaydedilen tercihler.
- Windows Spotify uygulaması için şarkı bilgisi, kapak, oynatma ve yalnızca Spotify’a özel ses kontrolü.
- Ajan, harita ve saldırı/savunma filtreleriyle uygulama içinde Tracker.gg lineup videoları.
- Son 5/10/20 maç, skor tabloları, ajan/harita istatistikleri ve maç sonu özeti.
- Kendi profilin için mevcut Riot oturumundan alınan veriler ve çevrimdışı önbellek.
- İmzalı otomatik güncellemeler.

[Windows kurulumunu indir](https://github.com/katsudc/katsu-tracker-releases/releases/latest/download/Katsu-Tracker-setup.exe)

İlk kurulumdan sonra yeni sürümler arka planda indirilir. Güncelleme hazır olduğunda VALORANT kapalıyken bildirim düğmesinden kurabilir veya tracker’ı sonraki açışında yükletebilirsin. Ayarların korunur.

Kendi profilindeki KDA / HS% / WR değerleri erişilebilen en fazla son 20 rekabetçi maça aittir. Veriler aşamalı yüklenir; tamamlanana kadar işlenen maç sayısı gösterilir. Tracker Score veya tüm zamanlar istatistikleri değildir. Spotify için Windows masaüstü uygulamasında bir şarkı açmak yeterlidir; tracker içinde hesap girişi gerekmez.

Oyuncu rütbeleri mevcut Riot oturumundaki maç verisi ve MMR yanıtlarından alınır. Bilinen bir rütbe yoksa tahmin yapılmaz: doğrulanmış derecesiz hesap ile rütbesi alınamayan hesap ayrı gösterilir. Profilin ilk kez bulunması için VALORANT ile bir kez bağlanmak gerekir. Kendi istatistiklerin en fazla 20 erişilebilir rekabetçi maçla sınırlıdır. Canlı oyuncu kartlarında KDA, ACS, HS%, WR veya geçmiş performans gösterilmez; rakip geçmişi sorgulanmaz. API anahtarı veya ek istatistik servisi hesabı gerekmez.

Bu GitHub deposu şu anda kurulum dosyaları ve güncellemeler içindir. Kaynak kod bu dağıtım deposunda henüz yayımlanmamıştır.

## English

A Windows desktop companion for VALORANT, maintained by katsu.

- Live player names, agents and ranks; existing weapon-skin views remain available.
- Per-map agent selection and automatic locking preferences.
- A compact timer triggered by the planted-spike HUD icon on the game monitor.
- Five accent themes, Turkish/English UI and saved preferences.
- Spotify desktop track metadata, artwork, playback controls and Spotify-only volume.
- In-app Tracker.gg lineup guides filtered by agent, map and attack/defense.
- Last 5/10/20 matches, scoreboards, agent/map statistics and post-match recap.
- Your own profile statistics from the current Riot session, with offline caching.
- Signed automatic updates.

[Download for Windows](https://github.com/katsudc/katsu-tracker-releases/releases/latest/download/Katsu-Tracker-setup.exe)

After the first installation, new versions download in the background. Install a ready update using its notification while VALORANT is closed, or let it install when you next open the tracker. Existing preferences are preserved.

Your own stats load progressively, with the processed match count shown until completion. KDA / HS% / WR cover up to 20 accessible recent Competitive matches, not lifetime statistics or Tracker Score. To enable music controls, play a track in the Windows Spotify desktop app; no Spotify sign-in inside the tracker is required.

Player ranks come from match data and MMR responses in the existing Riot session. Missing rank information is not guessed: confirmed unranked accounts and unavailable ranks are shown separately. Connect to VALORANT once to identify your profile. Your own statistics cover up to 20 accessible Competitive matches. Live player cards do not display KDA, ACS, HS%, WR or past performance; opponent history is not queried. No API key or external statistics account is required.

This GitHub repository currently distributes installers and updates. Source code has not yet been published in this distribution repository.

## License / Lisans

Built upon [ruwiss/valorant-tracker](https://github.com/ruwiss/valorant-tracker), under the MIT license. Original copyright notices are retained; modifications are credited to katsu. See LICENSE.txt in each release.

Temel proje MIT lisanslıdır. Özgün telif bildirimleri korunur; katsu’nun değişiklikleri ayrıca belirtilir. Her sürümde LICENSE.txt bulunur.

Not affiliated with Riot Games, Spotify or Tracker Network. / Riot Games, Spotify veya Tracker Network ile bağlantılı değildir.
