# Third-party notices

The Game Bar proxy-marshaling registrations in `widget/Package.appxmanifest` are based on Microsoft's XboxGameBarSamples project, used under the MIT license:

Copyright (c) Microsoft Corporation.

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

The Microsoft Game Bar SDK is a separate Microsoft component under its own license, included as `Microsoft-GameBar-SDK-LICENSE.txt` in the widget package. The project's MIT license does not replace Microsoft's SDK terms.

## Microsoft Visual C++ UWP runtime framework

The desktop installer includes the unmodified, Microsoft-signed retail x64 `Microsoft.VCLibs.140.00` framework (14.0.33519.0), a required dependency of the C++/CX widget. It is extracted from Microsoft's Visual Studio CRT Appx redistributable payload. Microsoft retains ownership of this runtime. `scripts/prepare_widget_runtime.ps1` records the pinned Microsoft download and integrity hashes. This is the UWP app-container framework, not the desktop VCRedist or UWPDesktop framework.

Reference: https://learn.microsoft.com/en-us/windows/uwp/cpp-and-winrt-apis/intro-to-using-cpp-with-winrt
