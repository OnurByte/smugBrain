<p align="center"><img src="assets/banner.svg" alt="secBrain" width="880"></p>

<p align="center"><a href="README.md">English</a> · <a href="LICENSE">Kamu malı</a> · <a href="CRYPTO-ANARCHY.md">Kripto-anarşi bildirisi</a></p>

**secBrain**, yapay zekâ ajanlarıyla çalışan ve hafızasını bir şirketten kiralamak istemeyen insanlar için ikinci beyindir. Markdown notlardan oluşan bir klasör ve küçük bir araç seti: Claude Code, Codex ve Hermes dünü hatırlar, doğru notu bulur, yedek alırken gizli bilgi sızdırmaz.

Metin editörü kullanabiliyorsan bunu da kullanabilirsin.

<p align="center"><img src="assets/card-what.svg" alt="Nedir" width="880"></p>

## Neden

Çoğu "yapay zekâ hafızası" ürünü notlarını başkasının sunucusunda tutar, şirket fikrini değiştirince bozulur. secBrain tek bir gerçek kaynak tutar: senin klasörün. Bütün indeksler silinip yeniden kurulabilir.

<p align="center"><img src="assets/card-how.svg" alt="Nasıl çalışır" width="880"></p>

## Sayılar

Yazarın referans vault'unda 2026-11-12'de ölçülen gerçek sayılar. Yuvarlanıp güzelleştirilmedi.

<p align="center"><img src="assets/card-stats.svg" alt="Sayılarla" width="880"></p>

Sıradaki hedefler. Bunlar **hedef**, sonuç değil:

<p align="center"><img src="assets/card-targets.svg" alt="Hedefler" width="880"></p>

## Yol haritası

<p align="center"><img src="assets/card-roadmap.svg" alt="Yol haritası" width="880"></p>

Plan: Markdown vault'u ve mevcut QMD aramasını koru, Rust çekirdek (`secbrain` CLI, geçmiş ve indeks için SQLite) ekle, "daha iyi" demeden önce her adımı benchmark ile kanıtla. Ayrıntı: [docs/roadmap.md](docs/roadmap.md).

## Başla

```bash
git clone https://github.com/OnurByte/secBrain.git my-brain
cd my-brain/template        # boş vault: klasörler + kurallar
```

`template/` klasörünü Obsidian'da aç. Ajanının uyduğu kurallar `template/CLAUDE.md` içinde. Otomasyon script'leri public olmadan önce temizleniyor, şimdilik kurulum bu kadar.

## Lisans

<p align="center"><img src="assets/card-license.svg" alt="Lisans" width="880"></p>

Tasarım dili: NASA Graphics Standards Manual NHB 1430.2 (1976). Kartları yeniden üretmek için: `python3 tools/make_cards.py`.
