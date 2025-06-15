use eframe::{egui, epaint::Color32};
use rfd::FileDialog;
use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, Write};
use std::sync::{Arc, Mutex};
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
    only_user: bool, // Nuevo campo
    min_numeric_len: usize,
    min_pass_len: usize,
    total_results: usize,
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
            only_user: false, // Nuevo campo
            min_numeric_len: 6,
            min_pass_len: 6,
            total_results: 0,
        }
    }
}

impl eframe::App for AppState {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Fondo negro
        let screen_rect = ctx.screen_rect();
        let painter = ctx.layer_painter(egui::LayerId::background());
        painter.rect_filled(
            screen_rect,
            Rounding::same(0.0),
            egui::Color32::BLACK,
        );

        // Difuminado rojo oscuro (simulado con un rectángulo semitransparente arriba)
        let height = screen_rect.height() * 0.4;
        let fade_rect = egui::Rect::from_min_max(
            screen_rect.left_top(),
            egui::pos2(screen_rect.right(), screen_rect.top() + height),
        );
        painter.rect_filled(
            fade_rect,
            Rounding::same(0.0),
            egui::Color32::from_rgba_unmultiplied(60, 0, 0, 120), // Rojo oscuro, semitransparente
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
            ui.checkbox(&mut self.only_user, "Solo user:pass"); // Nuevo

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
                            self.only_user, // Nuevo
                            self.min_numeric_len,
                            self.min_pass_len,
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

fn extract_line(line: &str, extract_after_colon: bool) -> Option<String> {
    if extract_after_colon {
        line.splitn(2, ':').nth(1).map(|s| s.trim_start().to_string())
    } else {
        Some(line.to_string())
    }
}

// Validación avanzada: permite marcar uno, varios o todos los tipos
fn is_valid_credential(
    s: &str,
    only_email: bool,
    only_numeric: bool,
    only_user: bool,
    min_numeric_len: usize,
    min_pass_len: usize,
) -> bool {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() < 2 {
        return false;
    }
    let user = parts[0].trim();
    let pass = parts[1].trim();
    if user.is_empty() || pass.is_empty() {
        return false;
    }
    let is_email = user.contains('@') && user.contains('.');
    let is_numeric = user.chars().all(|c| c.is_ascii_digit())
        && user.len() >= min_numeric_len
        && pass.len() >= min_pass_len;
    let is_user = user.chars().all(|c| c.is_ascii_alphanumeric()) && !is_email && !is_numeric;

    // Si no se marca ninguna opción, acepta todos los tipos válidos
    if !only_email && !only_numeric && !only_user {
        return is_email || is_numeric || is_user;
    }
    // Si se marca más de una opción, acepta cualquiera de las seleccionadas
    (only_email && is_email) ||
    (only_numeric && is_numeric) ||
    (only_user && is_user)
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
    only_user: bool, // Nuevo
    min_numeric_len: usize,
    min_pass_len: usize,
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
    let results = Arc::new(Mutex::new(Vec::new()));
    let (tx, rx) = mpsc::channel();

    let chunk_size = (paths.len() + num_threads - 1) / num_threads;
    let mut handles = vec![];

    for chunk in paths.chunks(chunk_size) {
        let tx = tx.clone();
        let keywords = keywords.to_owned();
        let chunk = chunk.to_owned();
        let extract_after_colon = extract_after_colon;
        let only_email = only_email;
        let only_numeric = only_numeric;
        let only_user = only_user;
        let min_numeric_len = min_numeric_len;
        let min_pass_len = min_pass_len;
        let handle = thread::spawn(move || {
            for path in chunk {
                let file = File::open(&path);
                if let Ok(file) = file {
                    let reader = io::BufReader::new(file);
                    for line in reader.lines() {
                        if let Ok(line) = line {
                            if keywords.iter().any(|kw| line.contains(kw)) {
                                if let Some(extracted) = extract_line(&line, extract_after_colon) {
                                    if is_valid_credential(&extracted, only_email, only_numeric, only_user, min_numeric_len, min_pass_len) {
                                        tx.send(extracted).unwrap();
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

    let mut processed = 0;
    let mut preview = Vec::new();
    for received in rx {
        if preview.len() < 10 {
            preview.push(received.clone());
        }
        results.lock().unwrap().push(received);
        processed += 1;
        progress_callback(processed as f32 / total_files.max(1) as f32, processed, total_files);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let output = results.lock().unwrap();
    let mut unique = HashSet::new();
    let mut deduped = Vec::new();
    for line in output.iter() {
        if unique.insert(line.clone()) {
            deduped.push(line.clone());
        }
    }
    let total_results = deduped.len();

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .append(append_mode)
        .truncate(!append_mode)
        .open(output_file)?;

    for line in deduped.iter() {
        writeln!(file, "{}", line)?;
    }

    let preview: Vec<String> = deduped.iter().take(10).cloned().collect();
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