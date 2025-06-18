use eframe::{egui, epaint::Color32};
use rfd::FileDialog;
use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, Write};
use std::thread;
use std::sync::mpsc;
use egui::epaint::Rounding;
use regex::Regex;

#[derive(PartialEq, Clone)]
enum Idioma {
    Espanol,
    English,
}

#[derive(PartialEq, Clone)]
enum CredType {
    All,
    Email,
    Numeric,
    User,
    Rut,
}

impl CredType {
    fn as_str(&self) -> &'static str {
        match self {
            CredType::All => "all",
            CredType::Email => "email",
            CredType::Numeric => "numeric",
            CredType::User => "user",
            CredType::Rut => "rut",
        }
    }
}

struct AppState {
    input_dir: Option<String>,
    output_dir: Option<String>,
    keywords: String,
    status_message: String,
    num_threads: usize,
    progress: f32,
    processed_files: usize,
    preview: Vec<String>,
    append_mode: bool,
    idioma: Idioma,
    error_message: String,
    cred_type: CredType,
    min_numeric_len: usize,
    min_pass_len: usize,
    min_user_len: usize,
    total_results: usize,
    min_rut_len: usize,
    max_rut_len: usize,
    only_valid_rut: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            input_dir: None,
            output_dir: None,
            keywords: String::new(),
            status_message: String::new(),
            num_threads: 4,
            progress: 0.0,
            processed_files: 0,
            preview: Vec::new(),
            append_mode: false,
            idioma: Idioma::Espanol,
            error_message: String::new(),
            cred_type: CredType::All,
            min_numeric_len: 7,
            min_pass_len: 4,
            min_user_len: 4,
            total_results: 0,
            min_rut_len: 7,
            max_rut_len: 8,
            only_valid_rut: false,
        }
    }
}

impl eframe::App for AppState {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let screen_rect = ctx.screen_rect();
        let painter = ctx.layer_painter(egui::LayerId::background());
        painter.rect_filled(screen_rect, Rounding::same(0.0), Color32::BLACK);
        let height = screen_rect.height() * 0.4;
        let fade_rect = egui::Rect::from_min_max(
            screen_rect.left_top(),
            egui::pos2(screen_rect.right(), screen_rect.top() + height),
        );
        painter.rect_filled(fade_rect, Rounding::same(0.0), Color32::from_rgb(60, 0, 0));

        let (lbl_select_dir, lbl_dir, lbl_select_out, lbl_out, lbl_keywords, lbl_threads, lbl_process, lbl_ready, _lbl_error, lbl_preview, _lbl_append, _lbl_copy, _lbl_lang, _lbl_files) = match self.idioma {
            Idioma::Espanol => (
                "Seleccionar carpeta", "Carpeta:", "Seleccionar carpeta de salida", "Carpeta salida:",
                "Palabras clave (opcional):", "Hilos:", "Procesar", "Listo", "Error", "Vista previa", "Agregar al archivo", "Copiar", "Idioma", "Archivos:"
            ),
            Idioma::English => (
                "Select folder", "Folder:", "Select output folder", "Output folder:",
                "Keywords (optional):", "Threads:", "Process", "Ready", "Error", "Preview", "Append to file", "Copy", "Language", "Files:"
            ),
        };

        egui::CentralPanel::default().show(ctx, |ui| {
            // Selector de idioma
            ui.horizontal(|ui| {
                ui.label("Idioma / Language:");
                ui.radio_value(&mut self.idioma, Idioma::Espanol, "Español");
                ui.radio_value(&mut self.idioma, Idioma::English, "English");
            });

            ui.horizontal(|ui| {
                if ui.button(lbl_select_dir).clicked() {
                    if let Some(dir) = FileDialog::new().pick_folder() {
                        self.input_dir = Some(dir.display().to_string());
                    }
                }
                ui.label(format!("{} {}", lbl_dir, self.input_dir.as_deref().unwrap_or("")));
            });

            ui.horizontal(|ui| {
                if ui.button(lbl_select_out).clicked() {
                    if let Some(dir) = FileDialog::new().pick_folder() {
                        self.output_dir = Some(dir.display().to_string());
                    }
                }
                ui.label(format!("{} {}", lbl_out, self.output_dir.as_deref().unwrap_or("")));
            });

            ui.horizontal(|ui| {
                ui.label(lbl_keywords);
                ui.text_edit_singleline(&mut self.keywords);
            });

            ui.horizontal(|ui| {
                ui.label("Tipo de credencial:");
                ui.radio_value(&mut self.cred_type, CredType::All, "Todos");
                ui.radio_value(&mut self.cred_type, CredType::Email, "Email");
                ui.radio_value(&mut self.cred_type, CredType::Rut, "RUT");
                ui.radio_value(&mut self.cred_type, CredType::Numeric, "Numérico");
                ui.radio_value(&mut self.cred_type, CredType::User, "Usuario");
            });

            if self.cred_type == CredType::Rut {
                ui.checkbox(&mut self.only_valid_rut, "Solo RUT válido");
            }

            ui.horizontal(|ui| {
                ui.label("Mín. largo numérico:");
                ui.add(egui::DragValue::new(&mut self.min_numeric_len).clamp_range(1..=32));
                ui.label("Mín. largo usuario:");
                ui.add(egui::DragValue::new(&mut self.min_user_len).clamp_range(1..=32));
                ui.label("Mín. largo pass:");
                ui.add(egui::DragValue::new(&mut self.min_pass_len).clamp_range(1..=32));
            });

            ui.horizontal(|ui| {
                ui.label(lbl_threads);
                ui.add(egui::Slider::new(&mut self.num_threads, 1..=16));
            });

            ui.horizontal(|ui| {
                if ui.button(lbl_process).clicked() {
                    self.status_message.clear();
                    self.error_message.clear();
                    self.progress = 0.0;
                    self.processed_files = 0;
                    self.total_results = 0;
                    self.preview.clear();

                    let input_dir = match &self.input_dir {
                        Some(d) => d.clone(),
                        None => {
                            self.error_message = "Selecciona una carpeta de entrada".to_string();
                            return;
                        }
                    };
                    let output_dir = match &self.output_dir {
                        Some(d) => d.clone(),
                        None => {
                            self.error_message = "Selecciona una carpeta de salida".to_string();
                            return;
                        }
                    };

                    let keywords: Vec<String> = self.keywords
                        .split(|c| c == ',' || c == ' ')
                        .filter(|s| !s.is_empty())
                        .map(|s| s.trim().to_lowercase())
                        .collect();

                    // Nombre de archivo de salida dinámico
                    let keyword_part = if !keywords.is_empty() {
                        keywords[0].replace('.', "_").replace('/', "_")
                    } else {
                        "all".to_string()
                    };
                    let filename = format!(
                        "{}_{}.txt",
                        keyword_part,
                        self.cred_type.as_str()
                    );
                    let output_file = format!("{}/{}", output_dir, filename);

                    let cred_type = self.cred_type.clone();
                    let only_valid_rut = self.only_valid_rut;
                    let min_numeric_len = self.min_numeric_len;
                    let min_pass_len = self.min_pass_len;
                    let min_user_len = self.min_user_len;
                    let min_rut_len = self.min_rut_len;
                    let max_rut_len = self.max_rut_len;
                    let num_threads = self.num_threads;
                    let append_mode = self.append_mode;

                    let processing = thread::spawn(move || {
                        process_files(
                            &input_dir,
                            &keywords,
                            &output_file,
                            num_threads,
                            cred_type,
                            only_valid_rut,
                            min_numeric_len,
                            min_pass_len,
                            min_user_len,
                            min_rut_len,
                            max_rut_len,
                            append_mode,
                        )
                    });

                    match processing.join().unwrap() {
                        Ok((preview, total_results)) => {
                            self.preview = preview;
                            self.total_results = total_results;
                            self.status_message = format!("{} ({})", lbl_ready, filename);
                        }
                        Err(e) => {
                            self.error_message = e.to_string();
                        }
                    }
                }
            });

            if !self.status_message.is_empty() {
                ui.label(format!("✅ {}", self.status_message));
            }
            if !self.error_message.is_empty() {
                ui.colored_label(Color32::RED, format!("❌ {}", self.error_message));
            }

            ui.separator();
            ui.label(format!("{} {}", lbl_preview, self.total_results));
            for line in &self.preview {
                ui.label(line);
            }
        });
    }
}

// --- FILTROS MEJORADOS ---

fn is_valid_email(user: &str) -> bool {
    if user.contains("MISSING-USER") { return false; }
    // Regex simple para email válido
    let re = Regex::new(r"^[a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\.[a-zA-Z0-9-.]+$").unwrap();
    re.is_match(user)
}

fn is_valid_numeric(user: &str, min_numeric_len: usize) -> bool {
    user.chars().all(|c| c.is_ascii_digit()) && user.len() >= min_numeric_len
}

fn is_valid_user(user: &str, min_user_len: usize) -> bool {
    // Solo letras, números, guion bajo y punto, no emails ni campos vacíos
    !user.contains('@')
        && !user.contains("MISSING-USER")
        && user.len() >= min_user_len
        && user.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
        && !user.chars().all(|c| c.is_ascii_digit())
}

fn rut_dv(num: &str) -> Option<char> {
    let mut sum = 0;
    let mut mul = 2;
    for c in num.chars().rev() {
        if let Some(d) = c.to_digit(10) {
            sum += d * mul;
            mul = if mul == 7 { 2 } else { mul + 1 };
        } else {
            return None;
        }
    }
    let res = 11 - (sum % 11);
    Some(match res {
        11 => '0',
        10 => 'k',
        x => std::char::from_digit(x, 10)?,
    })
}

fn is_rut(
    user: &str,
    require_valid_dv: bool,
    min_rut_len: usize,
    max_rut_len: usize,
) -> bool {
    if user.contains("MISSING-USER") { return false; }
    if let Some(idx) = user.find('-') {
        let (num, dv) = user.split_at(idx);
        let dv = dv[1..].to_ascii_lowercase();
        if num.len() < min_rut_len || num.len() > max_rut_len { return false; }
        if !num.chars().all(|c| c.is_ascii_digit()) { return false; }
        if require_valid_dv {
            if let Some(calc_dv) = rut_dv(num) {
                return dv == calc_dv.to_string();
            } else {
                return false;
            }
        }
        return dv == "k" || dv.chars().all(|c| c.is_ascii_digit());
    }
    if user.len() < min_rut_len + 1 || user.len() > max_rut_len + 1 {
        return false;
    }
    let (num, dv) = user.split_at(user.len() - 1);
    let dv = dv.to_ascii_lowercase();
    if !num.chars().all(|c| c.is_ascii_digit()) { return false; }
    if require_valid_dv {
        if let Some(calc_dv) = rut_dv(num) {
            return dv == calc_dv.to_string();
        } else {
            return false;
        }
    }
    dv == "k" || dv.chars().all(|c| c.is_ascii_digit())
}

fn is_valid_credential(
    user: &str,
    cred_type: &CredType,
    only_valid_rut: bool,
    min_numeric_len: usize,
    min_user_len: usize,
    min_rut_len: usize,
    max_rut_len: usize,
) -> bool {
    match cred_type {
        CredType::All => {
            is_valid_email(user)
                || is_valid_numeric(user, min_numeric_len)
                || is_valid_user(user, min_user_len)
                || is_rut(user, only_valid_rut, min_rut_len, max_rut_len)
        }
        CredType::Email => is_valid_email(user),
        CredType::Numeric => is_valid_numeric(user, min_numeric_len),
        CredType::User => is_valid_user(user, min_user_len),
        CredType::Rut => is_rut(user, only_valid_rut, min_rut_len, max_rut_len),
    }
}

fn process_files(
    input_directory: &str,
    keywords: &[String],
    output_file: &str,
    num_threads: usize,
    cred_type: CredType,
    only_valid_rut: bool,
    min_numeric_len: usize,
    min_pass_len: usize,
    min_user_len: usize,
    min_rut_len: usize,
    max_rut_len: usize,
    append_mode: bool,
) -> io::Result<(Vec<String>, usize)> {
    let allowed_exts = ["txt", "csv", "log"];
    let paths: Vec<_> = fs::read_dir(input_directory)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            if let Some(ext) = e.path().extension() {
                allowed_exts.iter().any(|&a| ext == a)
            } else {
                false
            }
        })
        .collect();

    if paths.is_empty() {
        return Ok((Vec::new(), 0));
    }

    let num_threads = num_threads.min(paths.len().max(1));
    let chunk_size = ((paths.len() + num_threads - 1) / num_threads).max(1);
    let mut handles = vec![];
    let (tx, rx) = mpsc::channel();

    for chunk in paths.chunks(chunk_size) {
        let tx = tx.clone();
        let chunk: Vec<_> = chunk.iter().map(|e| e.path()).collect();
        let keywords = keywords.to_owned();
        let cred_type = cred_type.clone();
        let only_valid_rut = only_valid_rut;
        let min_numeric_len = min_numeric_len;
        let min_pass_len = min_pass_len;
        let min_user_len = min_user_len;
        let min_rut_len = min_rut_len;
        let max_rut_len = max_rut_len;

        handles.push(thread::spawn(move || {
            for path in chunk {
                if let Ok(file) = File::open(&path) {
                    let reader = io::BufReader::new(file);
                    for line in reader.lines().flatten() {
                        let parts: Vec<&str> = line.splitn(3, ':').collect();
                        if parts.len() < 3 { continue; }
                        let user = parts[1].trim();
                        let pass = parts[2].trim();
                        if pass.len() < min_pass_len { continue; }
                        if !keywords.is_empty() && !keywords.iter().any(|k| line.to_lowercase().contains(k)) {
                            continue;
                        }
                        if is_valid_credential(
                            user,
                            &cred_type,
                            only_valid_rut,
                            min_numeric_len,
                            min_user_len,
                            min_rut_len,
                            max_rut_len,
                        ) {
                            let extracted = format!("{}:{}", user, pass);
                            if let Err(e) = tx.send(extracted) {
                                eprintln!("Error enviando resultado: {:?}", e);
                            }
                        }
                    }
                }
            }
        }));
    }

    drop(tx);

    for handle in handles {
        let _ = handle.join();
    }

    let mut unique = HashSet::new();
    let mut deduped = Vec::new();
    for received in rx {
        if unique.insert(received.clone()) {
            deduped.push(received);
        }
    }

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .append(append_mode)
        .truncate(!append_mode)
        .open(output_file)?;

    for line in &deduped {
        writeln!(file, "{}", line)?;
    }

    let total_results = deduped.len();
    let preview = deduped.iter().take(10).cloned().collect();

    Ok((preview, total_results))
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "ULP Extractor",
        options,
        Box::new(|_cc| Box::new(AppState::default())),
    )
}