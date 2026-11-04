//! TM AutoClip yerel yaması — SoundCraft (4 Kas 2026). Upstream'e dokunuş: `lib.rs` → `pub mod tm;`, `app.rs` → her karede
//! `tm::kare`, `menus.rs` → Dosya menüsüne öğe, `apps/soundcraft/src/main.rs` → kapı.
//!
//! - Akış kipinde (TM AutoClip'in Render Kuyruğu'ndan açılış) Dosya ▸ "Bitir ve TM AutoClip'e gönder": oturum kaydedilir (TM
//!   AutoClip kesimleri ve bellek konumlarını kaydedilen oturumdan okur), karışım oturumun beklenen çıktı yoluna bounce edilir
//!   (upstream'in kendi Bounce Mix işi, ilerleme penceresiyle), bitince `teslim.json` arka planda yazılır ve program kapanır.
//! - Düz kipte (oturumsuz ya da `kip = duz`) hiçbir şey görünmez.

pub mod oturum;

use std::path::PathBuf;
use std::sync::Mutex;
use std::thread::JoinHandle;
use std::time::SystemTime;

use serde_json::{Value, json};

use crate::SoundApp;

/// Bu programın oturumdaki adı.
pub const PROGRAM: &str = "soundcraft";

enum Durum {
    Bos,
    /// Bounce sürüyor: (çıktı, başlangıç).
    Bounce(PathBuf, SystemTime),
    /// Teslim arka planda yazılıyor.
    Teslim(JoinHandle<Result<PathBuf, String>>),
}

static DURUM: Mutex<Durum> = Mutex::new(Durum::Bos);

fn durum<R>(f: impl FnOnce(&mut Durum) -> R) -> R {
    f(&mut DURUM.lock().unwrap_or_else(std::sync::PoisonError::into_inner))
}

/// Dosya menüsünün sonuna bizim öğe (yalnız akış kipinde).
pub fn dosya_menusu(app: &mut SoundApp, ui: &mut egui::Ui) {
    let Some(o) = oturum::akis() else { return };
    let mesgul = durum(|d| !matches!(d, Durum::Bos)) || app.bounce.is_some();
    ui.separator();
    if ui.add_enabled(!mesgul, egui::Button::new(o.metin("sesGonder", "Bitir ve TM AutoClip'e gönder"))).clicked() {
        if let Err(e) = baslat(app) {
            log::error!("TM AutoClip: {e}");
            app.ui.status = format!("TM AutoClip'e gönderilemedi: {e}");
        }
        ui.close();
    }
}

/// Oturumu kaydeder ve karışımı beklenen çıktıya bounce etmeye başlar.
pub fn baslat(app: &mut SoundApp) -> Result<Value, String> {
    let o = oturum::akis().ok_or("TM AutoClip akış oturumu yok")?;
    if durum(|d| !matches!(d, Durum::Bos)) || app.bounce.is_some() {
        return Err("bounce zaten sürüyor".into());
    }
    let (_, yol) = o.cikti.clone().ok_or("oturumda çıktı yolu yok")?;
    // TM AutoClip kesimleri ve bellek konumlarını kaydedilen oturumdan okur: önce kaydet.
    app.run("session.save", json!({})).map_err(|e| format!("oturum kaydedilemedi: {e}"))?;
    let baslangic = SystemTime::now();
    crate::bounce::start(app, &json!({"path": yol.to_string_lossy()}));
    if app.bounce.is_none() {
        return Err(format!("bounce başlamadı: {}", app.ui.status));
    }
    log::info!("TM AutoClip: bounce başladı ({})", yol.display());
    durum(|d| *d = Durum::Bounce(yol.clone(), baslangic));
    Ok(json!({"path": yol.to_string_lossy()}))
}

/// Her kare: bounce bittiyse teslim arka planda yazılır; teslim bitince program kapanır.
pub fn kare(app: &mut SoundApp, ctx: &egui::Context) {
    let Some(o) = oturum::akis() else { return };
    let bitti = durum(|d| match std::mem::replace(d, Durum::Bos) {
        Durum::Teslim(is) if is.is_finished() => Some(is.join()),
        diger => {
            *d = diger;
            None
        }
    });
    if let Some(sonuc) = bitti {
        match sonuc {
            Ok(Ok(_)) => {
                app.ui.status = "TM AutoClip'e gönderildi".into();
                // Oturum bounce'tan önce kaydedildi; kaydet sorusu çıkmadan kapanır.
                app.quit_requested = true;
            }
            Ok(Err(e)) => {
                log::error!("TM AutoClip: {e}");
                app.ui.status = format!("TM AutoClip'e gönderilemedi: {e}");
            }
            Err(_) => {
                log::error!("TM AutoClip: teslim yazan iş çöktü");
                app.ui.status = "TM AutoClip'e gönderilemedi".into();
            }
        }
        return;
    }
    let bekleyen = durum(|d| match d {
        Durum::Bounce(yol, t) if app.bounce.is_none() => Some((yol.clone(), *t)),
        Durum::Teslim(_) => {
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
            None
        }
        _ => None,
    });
    let Some((yol, baslangic)) = bekleyen else { return };
    // Bounce bitti, iptal edildi ya da hata verdi: yalnız bu bounce'un yazdığı dolu dosya teslim edilir.
    let yeni = std::fs::metadata(&yol)
        .ok()
        .filter(|m| m.len() > 0)
        .and_then(|m| m.modified().ok())
        .is_some_and(|t| t.checked_add(std::time::Duration::from_secs(2)).is_some_and(|t| t >= baslangic));
    if yeni {
        app.ui.status = "TM AutoClip için çıktı doğrulanıyor…".into();
        durum(|d| *d = Durum::Teslim(oturum::teslim_arka_planda(o, yol)));
        ctx.request_repaint_after(std::time::Duration::from_millis(200));
    } else {
        durum(|d| *d = Durum::Bos);
        log::warn!("TM AutoClip: bounce çıktı vermedi ({})", app.ui.status);
        app.ui.status = format!("TM AutoClip'e gönderilmedi: {}", app.ui.status);
    }
}
