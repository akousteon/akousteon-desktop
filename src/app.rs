use crate::components::{
    to_display, to_display_h_m_s, Order, Speakers, Speech, TSpeakers, Timespan,
};
use egui::ScrollArea;
use rfd::{MessageDialog, MessageDialogResult};
use std::{collections::VecDeque, future::Future};
use web_time::Duration;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct TemplateApp {
    timespan: Timespan,
    speeches: Vec<Speech>,
    want_speak: Order,
    speakers: Speakers,
    categories: Vec<String>,
    new_speaker: String,
    categorie_new_speaker: String,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            timespan: Timespan::new(),
            speeches: Vec::new(),
            want_speak: VecDeque::new(),
            speakers: Vec::new(),
            categories: Vec::new(),
            new_speaker: String::new(),
            categorie_new_speaker: String::new(),
        }
    }
}

impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }

    pub fn export_speeches_to_csv_file(&self) {
        let mut string_to_export = String::new();
        for speech in self.speeches.iter() {
            string_to_export.push_str(&speech.export_to_csv());
            string_to_export.push('\n');
        }

        let task = rfd::AsyncFileDialog::new().save_file();
        let contents = string_to_export.clone();
        execute(async move {
            let file = task.await;
            if let Some(file) = file {
                _ = file.write(contents.as_bytes()).await;
            }
        });
    }
}

impl eframe::App for TemplateApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Using columns to work around layouts limits
            ui.columns(3, |columns| {
                columns[0].vertical(|ui| {
                    ui.label(format!(
                        "Tour de parole en cours : {}",
                        self.speakers.current_speaker(&self.want_speak).0
                    ));
                    ui.label(format!(
                        "Tour de parole suivant : {}",
                        self.speakers.next_speaker(&self.want_speak).0
                    ));
                    ui.label("Ordre des tours de parole");
                    for index_speaker in self.want_speak.clone() {
                        ui.horizontal(|ui| {
                            if ui.button("x").clicked() {
                                self.want_speak.remove(index_speaker);
                            }
                            ui.label(self.speakers.get_speaker(index_speaker).0);
                        });
                    }

                    ui.label("Ajouter une personne dans la liste des tours de parole");
                    for (i, speaker) in self.speakers.clone().iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(speaker.0.clone());
                            if ui.button("+").clicked() {
                                self.speakers
                                    .speaker_wants_to_speak(i, &mut self.want_speak);
                            }

                            if ui.button("x").clicked() {
                                self.speakers.delete_speaker(i, &mut self.want_speak);
                            }
                        });
                    }

                    ui.label("Ajouter un·e orateur·ice");
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut self.new_speaker);

                        egui::ComboBox::from_id_source("new_speaker")
                            .selected_text(&self.categorie_new_speaker)
                            .show_ui(ui, |ui| {
                                for category in self.categories.iter() {
                                    if ui.selectable_label(false, category).clicked() {
                                        self.categorie_new_speaker = category.to_string();
                                    };
                                }
                            });

                        if ui.button("+").clicked() {
                            self.speakers.add_speaker((
                                self.new_speaker.clone(),
                                self.categorie_new_speaker.clone(),
                            ));
                        }
                    });
                });
                columns[1].vertical_centered(|ui| {
                    ui.horizontal(|ui| {
                        ui.style_mut().text_styles.insert(
                            egui::TextStyle::Button,
                            egui::FontId::new(24.0, eframe::epaint::FontFamily::Proportional),
                        );

                        ui.style_mut().text_styles.insert(
                            egui::TextStyle::Body,
                            egui::FontId::new(24.0, eframe::epaint::FontFamily::Proportional),
                        );

                        ui.label(to_display(self.timespan.elapsed()))
                            .ctx
                            .request_repaint_after(Duration::new(0, 10_000_000));

                        let label_button_play = {
                            if self.timespan.is_running() {
                                "⏸"
                            } else {
                                "⏵"
                            }
                        };
                        if ui.button(label_button_play).clicked() {
                            self.timespan.start_or_stop();
                        }

                        if ui
                            .add_enabled(
                                self.timespan.elapsed().as_secs() > 0,
                                egui::Button::new("+"),
                            )
                            .clicked()
                        {
                            self.timespan.stop();
                            self.speeches.push(Speech {
                                duration: self.timespan.elapsed,
                                category: self.speakers.current_speaker(&self.want_speak).1,
                            });
                            self.timespan.reset();
                            self.speakers.speaker_spoke(&mut self.want_speak);
                        }
                    });
                    ScrollArea::vertical().show(ui, |ui| {
                        for (i, speech) in self.speeches.clone().into_iter().enumerate().rev() {
                            ui.horizontal(|ui| {
                                if ui.button("x").clicked() {
                                    self.speeches.remove(i);
                                }
                                ui.label(to_display(speech.duration));

                                egui::ComboBox::from_id_source(i)
                                    .selected_text(&speech.category)
                                    .show_ui(ui, |ui| {
                                        for category in self.categories.iter() {
                                            if ui.selectable_label(false, category).clicked() {
                                                self.speeches.get_mut(i).unwrap().category =
                                                    category.to_string();
                                            };
                                        }
                                    });
                            });
                        }
                    });
                });
                columns[2].vertical_centered(|ui| {
                    ui.horizontal(|ui| {
                        if ui.button("Export").clicked() {
                            self.export_speeches_to_csv_file();
                        }
                        if ui.button("Clear").clicked() {
                            let dialog = MessageDialog::new()
                                .set_title("Confirmation de l'effacement")
                                .set_description("Confirmez l'effacement des tours de parole")
                                .set_buttons(rfd::MessageButtons::YesNo);
                            let result = dialog.show();
                            match result {
                                MessageDialogResult::Yes | MessageDialogResult::Ok => {
                                    self.timespan.reset();
                                    self.speeches.clear();
                                }
                                _ => {}
                            }
                        }
                    });

                    ui.add_space(20.);
                    ui.label("Une catégorie par ligne");
                    let mut s_categories = self.categories.join("\n");
                    if ui.text_edit_multiline(&mut s_categories).changed() {
                        self.categories = s_categories
                            .split('\n')
                            .map(|x| x.to_owned())
                            .collect::<Vec<String>>();
                    }

                    ui.label("Temps total par catégorie");
                    for category in self.categories.iter() {
                        let sum: Duration = self
                            .speeches
                            .iter()
                            .filter(|x| x.category == *category)
                            .map(|x| x.duration)
                            .sum();

                        ui.label(format!(
                            "{} prises de paroles des {} : {}",
                            self.speeches
                                .iter()
                                .filter(|x| x.category == *category)
                                .count(),
                            category,
                            to_display_h_m_s(sum)
                        ));
                    }
                });
            });
        });
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn execute<F: Future<Output = ()> + Send + 'static>(f: F) {
    // this is stupid... use any executor of your choice instead
    std::thread::spawn(move || futures::executor::block_on(f));
}
#[cfg(target_arch = "wasm32")]
fn execute<F: Future<Output = ()> + 'static>(f: F) {
    wasm_bindgen_futures::spawn_local(f);
}
