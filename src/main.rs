use eframe::{egui, epaint::Color32};
use rfd::FileDialog;
use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, Write};
use std::thread;
use std::sync::mpsc;
use egui::epaint::Rounding;

#[derive(PartialEq)]
enum Idioma {
    Espanol,
    English,
}

struct AppState {
    input_dir: Option<String>,
    output_file: Option<String>,
    keywords: String,
    result: String,
    num_threads: usize,
    extract_after_colon: bool,
    progress: f32,
    processed_files: usize,
    total_files: usize,
    preview: Vec<String>,
    append_mode: bool,
    idioma: Idioma,
    error_message: String,
    only_email: bool,
    only_numeric: bool,
    only_user: bool,
    min_numeric_len: usize,
    min_pass_len: usize,
    total_results: usize,
    only_rut: bool,
    only_valid_rut: bool,
    min_rut_len: usize,
    max_rut_len: usize,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            input_dir: None,
            output_file: None,
            keywords: String::new(),
            result: String::new(),
            num_threads: num_cpus::get(),
            extract_after_colon: false,
            progress: 0.0,
            processed_files: 0,
            total_files: 0,
            preview: Vec::new(),
            append_mode: false,
            idioma: Idioma::Espanol,
            error_message: String::new(),
            only_email: false,
            only_numeric: false,
            only_user: false,
            min_numeric_len: 6,
            min_pass_len: 6,
            total_results: 0,
            only_rut: false,
            only_valid_rut: false,
            min_rut_len: 7,
            max_rut_len: 8,
        }
    }
}

impl eframe::App for AppState {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Fondo negro y difuminado rojo oscuro
        let screen_rect = ctx.screen_rect();
        let painter = ctx.layer_painter(egui::LayerId::background());
        painter.rect_filled(
            screen_rect,
            Rounding::same(0.0),
            egui::Color32::BLACK,
        );
        let height = screen_rect.height() * 0.4;
        let fade_rect = egui::Rect::from_min_max(
            screen_rect.left_top(),
            egui::pos2(screen_rect.right(), screen_rect.top() + height),
        );
        painter.rect_filled(
            fade_rect,
            Rounding::same(0.0),
            egui::Color32::from_rgba_unmultiplied(60, 0, 0, 120),
        );

        let (lbl_select_dir, lbl_dir, lbl_select_out, lbl_out, lbl_keywords, lbl_extract, lbl_threads, lbl_process, lbl_ready, lbl_error, lbl_preview, lbl_append, lbl_copy, lbl_lang, lbl_files, _lbl_progress) = match self.idioma {
            Idioma::Espanol => (
                "Seleccionar directorio de entrada",
                "Directorio",
                "Seleccionar archivo de salida",
                "Archivo de salida",
                "Palabras clave (separadas por coma):",
                "Extraer solo lo después de ':'",
                "Hilos (threads)",
                "Procesar",
                "¡Listo! Archivo generado.",
                "Error",
                "Vista previa (primeras 10 líneas):",
                "Agregar al final del archivo",
                "Copiar resultados",
                "Idioma",
                "Archivos procesados",
                "Progreso",
            ),
            Idioma::English => (
                "Select input directory",
                "Directory",
                "Select output file",
                "Output file",
                "Keywords (comma separated):",
                "Extract only after ':'",
                "Threads",
                "Process",
                "Done! File generated.",
                "Error",
                "Preview (first 10 lines):",
                "Append to output file",
                "Copy results",
                "Language",
                "Files processed",
                "Progress",
            ),
        };

        egui::CentralPanel::default().show(ctx, |ui| {
            // Language selector
            ui.horizontal(|ui| {
                ui.label(lbl_lang);
                if ui.selectable_label(self.idioma == Idioma::Espanol, "Español").clicked() {
                    self.idioma = Idioma::Espanol;
                }
                if ui.selectable_label(self.idioma == Idioma::English, "English").clicked() {
                    self.idioma = Idioma::English;
                }
            });

            if ui.button(lbl_select_dir).clicked() {
                if let Some(dir) = FileDialog::new().pick_folder() {
                    self.input_dir = Some(dir.display().to_string());
                }
            }
            if let Some(ref dir) = self.input_dir {
                ui.label(format!("{lbl_dir}: {dir}"));
            }

            if ui.button(lbl_select_out).clicked() {
                if let Some(file) = FileDialog::new().save_file() {
                    self.output_file = Some(file.display().to_string());
                }
            }
            if let Some(ref file) = self.output_file {
                ui.label(format!("{lbl_out}: {file}"));
            }

            ui.horizontal(|ui| {
                ui.label(lbl_keywords);
                ui.text_edit_singleline(&mut self.keywords);
            });

            ui.checkbox(&mut self.extract_after_colon, lbl_extract);
            ui.checkbox(&mut self.append_mode, lbl_append);
            ui.checkbox(&mut self.only_email, "Solo email:pass");
            ui.checkbox(&mut self.only_numeric, "Solo num:pass");
            ui.checkbox(&mut self.only_user, "Solo user:pass");
            ui.checkbox(&mut self.only_rut, "Solo rut:pass");
            if self.only_rut {
                ui.checkbox(&mut self.only_valid_rut, "Solo RUTs con dígito verificador válido");
                ui.horizontal(|ui| {
                    ui.label("Mínimo largo de número:");
                    ui.add(egui::Slider::new(&mut self.min_rut_len, 6..=8));
                    ui.label("Máximo largo de número:");
                    ui.add(egui::Slider::new(&mut self.max_rut_len, self.min_rut_len..=8));
                });
            }

            if self.only_numeric {
                ui.horizontal(|ui| {
                    ui.label("Mínimo largo de número:");
                    ui.add(egui::Slider::new(&mut self.min_numeric_len, 1..=20));
                    ui.label("Mínimo largo de password:");
                    ui.add(egui::Slider::new(&mut self.min_pass_len, 1..=64));
                });
            }

            ui.add(egui::Slider::new(&mut self.num_threads, 1..=num_cpus::get())
                .text(lbl_threads));

            if ui.button(lbl_process).clicked() {
                self.result.clear();
                self.error_message.clear();
                self.preview.clear();
                self.progress = 0.0;
                self.processed_files = 0;
                if let (Some(ref dir), Some(ref out)) = (&self.input_dir, &self.output_file) {
                    let keywords: Vec<String> = self.keywords
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    if !keywords.is_empty() {
                        match process_files(
                            dir,
                            &keywords,
                            out,
                            self.num_threads,
                            self.extract_after_colon,
                            self.append_mode,
                            self.only_email,
                            self.only_numeric,
                            self.only_user,
                            self.only_rut,
                            self.only_valid_rut,
                            self.min_numeric_len,
                            self.min_pass_len,
                            self.min_rut_len,
                            self.max_rut_len,
                            |progress, processed, total| {
                                self.progress = progress;
                                self.processed_files = processed;
                                self.total_files = total;
                                ctx.request_repaint();
                            },
                        ) {
                            Ok((preview, total_results)) => {
                                self.result = lbl_ready.to_string();
                                self.preview = preview;
                                self.total_results = total_results;
                            }
                            Err(e) => {
                                self.error_message = format!("{lbl_error}: {e}");
                            }
                        }
                    } else {
                        self.error_message = match self.idioma {
                            Idioma::Espanol => "Ingrese al menos una palabra clave.".to_string(),
                            Idioma::English => "Enter at least one keyword.".to_string(),
                        };
                    }
                } else {
                    self.error_message = match self.idioma {
                        Idioma::Espanol => "Seleccione directorio y archivo de salida.".to_string(),
                        Idioma::English => "Select directory and output file.".to_string(),
                    };
                }
            }

            if self.total_files > 0 {
                ui.add(egui::ProgressBar::new(self.progress).show_percentage());
                ui.label(format!("{lbl_files}: {}/{}", self.processed_files, self.total_files));
            }

            if !self.result.is_empty() {
                ui.colored_label(Color32::DARK_GREEN, &self.result);
            }
            if !self.error_message.is_empty() {
                ui.colored_label(Color32::RED, &self.error_message);
            }

            if !self.preview.is_empty() {
                ui.separator();
                ui.label(lbl_preview);
                for line in self.preview.iter().take(10) {
                    ui.monospace(line);
                }
                if ui.button(lbl_copy).clicked() {
                    ui.output_mut(|o| o.copied_text = self.preview.join("\n"));
                }
            }

            if self.total_results > 0 {
                ui.colored_label(Color32::BLACK, format!("Resultados únicos: {}", self.total_results));
            }

            if ui.button("Limpiar todo").clicked() {
                *self = AppState::default();
            }
        });
    }
}

fn detect_separator(line: &str) -> Option<&'static str> {
    if line.contains(':') {
        Some(":")
    } else if line.contains('|') {
        Some("|")
    } else if line.contains(';') {
        Some(";")
    } else {
        None
    }
}

fn extract_line(line: &str, extract_after_colon: bool) -> Option<String> {
    let sep = detect_separator(line)?;
    let parts: Vec<&str> = line.splitn(2, sep).collect();
    if parts.len() < 2 {
        return None;
    }
    let user = parts[0].trim();
    let pass = parts[1].trim();
    if extract_after_colon {
        Some(pass.to_string())
    } else {
        Some(format!("{}:{}", user, pass)) // Siempre retorna user:pass
    }
}

fn is_valid_credential(
    user: &str,
    only_email: bool,
    only_numeric: bool,
    only_user: bool,
    only_rut: bool,
    only_valid_rut: bool,
    min_numeric_len: usize,
    min_rut_len: usize,
    max_rut_len: usize,
) -> bool {
    let is_email = user.contains('@') && user.contains('.');
    let is_numeric = user.chars().all(|c| c.is_ascii_digit()) && user.len() >= min_numeric_len;
    let is_user = user.chars().all(|c| c.is_ascii_alphanumeric()) && !is_email && !is_numeric;
    let is_rut_user = is_rut(user, only_valid_rut, min_rut_len, max_rut_len);

    if !only_email && !only_numeric && !only_user && !only_rut {
        return is_email || is_numeric || is_user || is_rut_user;
    }
    (only_email && is_email)
        || (only_numeric && is_numeric)
        || (only_user && is_user)
        || (only_rut && is_rut_user)
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
    // Si tiene guion, separar normalmente
    if let Some(idx) = user.find('-') {
        let (num, dv) = user.split_at(idx);
        let dv = dv[1..].to_ascii_lowercase(); // <-- CORREGIDO: dv es String
        if num.len() < min_rut_len || num.len() > max_rut_len { return false; }
        if !num.chars().all(|c| c.is_ascii_digit()) { return false; }
        if require_valid_dv {
            if let Some(calc_dv) = rut_dv(num) {
                return dv == calc_dv.to_string(); // <-- Ahora compara String == String
            } else {
                return false;
            }
        }
        return dv == "k" || dv.chars().all(|c| c.is_ascii_digit());
    }

    // Si no tiene guion, último carácter es el dígito verificador
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

fn process_files<F>(
    input_directory: &str,
    keywords: &[String],
    output_file: &str,
    num_threads: usize,
    extract_after_colon: bool,
    append_mode: bool,
    only_email: bool,
    only_numeric: bool,
    only_user: bool,
    only_rut: bool,
    only_valid_rut: bool,
    min_numeric_len: usize,
    min_pass_len: usize,
    min_rut_len: usize,
    max_rut_len: usize,
    mut progress_callback: F,
) -> io::Result<(Vec<String>, usize)>
where
    F: FnMut(f32, usize, usize),
{
    let allowed_exts = ["txt", "csv", "log"];
    let paths = fs::read_dir(input_directory)?
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.path().extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| allowed_exts.contains(&ext))
                .unwrap_or(false)
        })
        .map(|entry| entry.path())
        .collect::<Vec<_>>();

    let total_files = paths.len();
    let (tx, rx) = mpsc::channel();

    let num_threads = num_threads.min(paths.len().max(1));
    let chunk_size = ((paths.len() + num_threads - 1) / num_threads).max(1);
    let mut handles = vec![];

    for chunk in paths.chunks(chunk_size) {
        let tx = tx.clone();
        let keywords = keywords.to_owned();
        let chunk = chunk.to_owned();
        let only_email = only_email;
        let only_numeric = only_numeric;
        let only_user = only_user;
        let only_rut = only_rut;
        let only_valid_rut = only_valid_rut;
        let min_numeric_len = min_numeric_len;
        let min_rut_len = min_rut_len;
        let max_rut_len = max_rut_len;
        let handle = thread::spawn(move || {
            for path in chunk {
                if let Ok(file) = File::open(&path) {
                    let reader = io::BufReader::new(file);
                    for line in reader.lines() {
                        if let Ok(line) = line {
                            if keywords.iter().any(|kw| line.contains(kw)) {
                                // Divide por el primer separador
                                if let Some(sep1) = detect_separator(&line) {
                                    let mut parts1 = line.splitn(2, sep1);
                                    let _url = parts1.next();
                                    if let Some(rest) = parts1.next() {
                                        // Divide el resto por el siguiente separador
                                        if let Some(sep2) = detect_separator(rest) {
                                            let mut parts2 = rest.splitn(2, sep2);
                                            let user_raw = parts2.next().unwrap_or("").trim();
                                            let pass = parts2.next().unwrap_or("").trim();
                                            // Limpia puntos del usuario para validación y salida
                                            let user = user_raw.replace('.', "");
                                            if !user.is_empty() && !pass.is_empty() {
                                                if is_valid_credential(
                                                    &user,
                                                    only_email,
                                                    only_numeric,
                                                    only_user,
                                                    only_rut,
                                                    only_valid_rut,
                                                    min_numeric_len,
                                                    min_rut_len,
                                                    max_rut_len,
                                                ) {
                                                    // Siempre salida user:pass (sin puntos en el user)
                                                    let extracted = format!("{}:{}", user, pass);
                                                    tx.send(extracted).unwrap();
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });
        handles.push(handle);
    }

    drop(tx);

    // Escribe todas las líneas válidas al archivo de salida (pueden estar duplicadas)
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .append(append_mode)
        .truncate(!append_mode)
        .open(output_file)?;

    let mut preview = Vec::new();
    let mut processed = 0;

    for received in rx {
        writeln!(file, "{}", received)?;
        if preview.len() < 10 {
            preview.push(received);
        }
        processed += 1;
        progress_callback(processed as f32 / total_files.max(1) as f32, processed, total_files);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // Deduplicar el archivo de salida al final
    let file = File::open(output_file)?;
    let reader = io::BufReader::new(file);
    let mut unique = HashSet::new();
    let mut deduped = Vec::new();
    for line in reader.lines() {
        if let Ok(line) = line {
            if unique.insert(line.clone()) {
                deduped.push(line);
            }
        }
    }

    // Sobrescribe el archivo solo con los resultados únicos
    let mut file = File::create(output_file)?;
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
        "Extractor",
        options,
        Box::new(|_cc| Box::new(AppState::default())),
    )
}