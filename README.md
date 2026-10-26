# katsu's Tracker

[English](#english) · [Türkçe](#türkçe)

## Türkçe

VALORANT için katsu tarafından geliştirilen Windows masaüstü arayüzü.

- Oyuncu kaplamaları ve rütbeleri; kendi profilin, maç geçmişin ve takım arkadaşlarının son rekabetçi maç istatistikleri.
- Haritaya göre ajan seçimi ve otomatik kilitleme tercihleri.
- Spike simgesini izleyen, oyun monitöründe görünen sade sayaç.
- Beş renk teması, Türkçe/İngilizce arayüz ve otomatik kaydedilen tercihler.
- Windows Spotify uygulaması için şarkı bilgisi, kapak, oynatma ve yalnızca Spotify’a özel ses kontrolü.
- Ajan, harita ve saldırı/savunma filtreleriyle uygulama içinde Tracker.gg lineup videoları.
- Son 5/10/20 maç, skor tabloları, ajan/harita istatistikleri ve maç sonu özeti.
- Çevrimdışı önbellek ve isteğe bağlı HenrikDev sağlayıcısı.
- Ayrı, varsayılanı kapalı Deneysel Rakip Analizi.
- İmzalı otomatik güncellemeler.

[Windows kurulumunu indir](https://github.com/katsudc/katsu-tracker-releases/releases/latest/download/Katsu-Tracker-setup.exe)

İlk kurulumdan sonra yeni sürümler arka planda indirilir. Güncelleme hazır olduğunda VALORANT kapalıyken bildirim düğmesinden kurabilir veya tracker’ı sonraki açışında yükletebilirsin. Ayarların korunur.

KDA / HS% / WR değerleri erişilebilen en fazla son 20 rekabetçi maça aittir. Veriler aşamalı yüklenir; tamamlanana kadar işlenen maç sayısı gösterilir. Tracker Score veya tüm zamanlar istatistikleri değildir. Spotify için Windows masaüstü uygulamasında bir şarkı açmak yeterlidir; tracker içinde hesap girişi gerekmez.

Varsayılan sağlayıcı mevcut Riot oturumudur. Profilin ilk kez bulunması için VALORANT ile bir kez bağlanmak gerekir. HenrikDev seçeneği kendi API anahtarını gerektirir; anahtar Windows kullanıcısına bağlı DPAPI ile saklanır. HenrikDev’e yalnızca kendi oyuncu kimliğin/istek bölgen gönderilir, Riot oturum anahtarların gönderilmez. RR değişimi/geçmişi yalnızca sağlayıcı döndürürse görünür; eksik alanlar uydurulmaz. Veriler en fazla 20 erişilebilir rekabetçi maçla sınırlıdır.

Deneysel Rakip Analizi ilk açılışta açık onay ister. Riot kurallarıyla çelişebilir; uygulama, API veya hesap yaptırımı riski taşıyabilir. Yalnızca oturumda kimliği görünen rakiplerin geçmişi elle sorgulanır; gizli kimlikler ve erişim kısıtları aşılmaz.

Bu GitHub deposu şu anda kurulum dosyaları ve güncellemeler içindir. Kaynak kod bu dağıtım deposunda henüz yayımlanmamıştır.

## English

A Windows desktop companion for VALORANT, maintained by katsu.

- Player skins and ranks; your profile, match history and recent Competitive stats for teammates.
- Per-map agent selection and automatic locking preferences.
- A compact timer triggered by the planted-spike HUD icon on the game monitor.
- Five accent themes, Turkish/English UI and saved preferences.
- Spotify desktop track metadata, artwork, playback controls and Spotify-only volume.
- In-app Tracker.gg lineup guides filtered by agent, map and attack/defense.
- Last 5/10/20 matches, scoreboards, agent/map statistics and post-match recap.
- Offline cache and an optional HenrikDev provider.
- Isolated Experimental Opponent Scouting, disabled by default.
- Signed automatic updates.

[Download for Windows](https://github.com/katsudc/katsu-tracker-releases/releases/latest/download/Katsu-Tracker-setup.exe)

After the first installation, new versions download in the background. Install a ready update using its notification while VALORANT is closed, or let it install when you next open the tracker. Existing preferences are preserved.

Stats load progressively, with the processed match count shown until completion. KDA / HS% / WR cover up to 20 accessible recent Competitive matches, not lifetime statistics or Tracker Score. To enable music controls, play a track in the Windows Spotify desktop app; no Spotify sign-in inside the tracker is required.

The default provider uses the current Riot session. Connect to VALORANT once to identify your profile. Optional HenrikDev requires your own API key, stored with Windows user-bound DPAPI. Only your own player identifier and requested region are sent to HenrikDev; Riot session tokens are never sent. Per-match RR appears only when supplied; missing fields remain unavailable. Data covers up to 20 accessible Competitive matches.

Experimental Opponent Scouting requires explicit first-use confirmation. It may conflict with Riot policies and carry application/API/account enforcement risk. Only opponents with visible identities in the current session can be manually queried; hidden identities and access restrictions are not bypassed.

This GitHub repository currently distributes installers and updates. Source code has not yet been published in this distribution repository.

## License / Lisans

Built upon [ruwiss/valorant-tracker](https://github.com/ruwiss/valorant-tracker), under the MIT license. Original copyright notices are retained; modifications are credited to katsu. See LICENSE.txt in each release.

Temel proje MIT lisanslıdır. Özgün telif bildirimleri korunur; katsu’nun değişiklikleri ayrıca belirtilir. Her sürümde LICENSE.txt bulunur.

Not affiliated with Riot Games, Spotify or Tracker Network. / Riot Games, Spotify veya Tracker Network ile bağlantılı değildir.
