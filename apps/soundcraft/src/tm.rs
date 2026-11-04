//! TM AutoClip yerel yaması — açılış kapısı (4 Kas 2026; FilmCraft/PhotoCraft/LightCraft'takiyle aynı kural). Oturum
//! (`TMAC_OTURUM` + `TMAC_BELIRTEC`) hiç verilmediyse program serbest açılır; oturum verildiyse ve geçersizse ileti gösterip 3
//! koduyla kapanır. Denetim `soundcraft_ui_egui::tm::oturum` içinde (beş yamalı programda ortak); ayrıntı TM AutoClip deposunda
//! `harici/craft-yamalar/YAMALAR.md`.

/// Verilen oturum geçerli değilse kullanıcıya söyler ve süreci 3 koduyla kapatır; oturum yoksa bir şey yapmaz.
pub fn kapi() {
    use soundcraft_ui_egui::tm::{PROGRAM, oturum};
    if let Err(sebep) = oturum::kapi_karari(PROGRAM) {
        log::warn!("TM AutoClip oturumu reddedildi: {sebep}");
        let metin = format!("{}\n\n({sebep})", oturum::RET_ILETISI);
        eprintln!("soundcraft: {metin}");
        let _ = rfd::MessageDialog::new().set_level(rfd::MessageLevel::Warning).set_title("SoundCraft — TM AutoClip").set_description(&metin).show();
        std::process::exit(3);
    }
}
