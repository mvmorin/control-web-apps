#![warn(clippy::all, rust_2018_idioms)]

#[allow(unused_imports)]
use basic_print::basic_print; // basic print for print-debugging


pub struct ControlApp {
    cur_app_idx: Option<usize>,
    apps: Vec<Box<dyn CentralApp>>,
}

trait CentralApp {
    fn draw_app(&mut self, ui: &mut egui::Ui);

    fn get_label(&self) -> &str;
}

impl ControlApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customized the look at feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        let apps: Vec<Box<dyn CentralApp>> = vec![
            Box::new(PolePos{
                label: "Pole Positioning".to_string(),
            }),
            Box::new(FreqResp{
                label: "Frequency Response".to_string(),
            }),
        ];

        ControlApp{cur_app_idx: None, apps }
    }


    fn top_bar(&mut self, ui: &mut egui::Ui) -> bool {
        #[allow(unused_mut)]
        let mut quit = false;

        ui.horizontal_wrapped(|ui| {
            ui.heading("Control Apps");
            ui.separator();

            for (idx, app) in self.apps.iter().enumerate() {
                let checked = match self.cur_app_idx {
                    Some(cur) => cur == idx,
                    None => false,
                };
                if ui.selectable_label(checked, app.get_label()).clicked() {
                    if checked {
                        self.cur_app_idx = None;
                    } else {
                        self.cur_app_idx = Some(idx);
                    }
                }
            }


            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                #[cfg(not(target_arch = "wasm32"))] // no quit on web pages!
                {
                    if ui.button("Quit").clicked() {
                        quit = true;
                    }
                    ui.separator();
                }
                egui::warn_if_debug_build(ui);
            });

        });

        return quit
    }

}


impl eframe::App for ControlApp {

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("app_selection_panel").show(ctx, |ui| {
            if self.top_bar(ui) {
                #[cfg(not(target_arch = "wasm32"))] // no quit on web pages!
                _frame.close();
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.centered_and_justified( |ui| {
                match self.cur_app_idx {
                    None => {ui.label("Select an application in the bar above.");},
                    Some(idx) => {self.apps[idx].draw_app(ui);},
                };
            });
        });
    }
}




struct PolePos{
    label: String,
}

impl CentralApp for PolePos {
    fn draw_app(&mut self, ui: &mut egui::Ui) {
        ui.label("In app, Pole Pos");
    }

    fn get_label(&self) -> &str {
        &self.label
    }
}




struct FreqResp{
    label: String,
}

impl CentralApp for FreqResp {
    fn draw_app(&mut self, ui: &mut egui::Ui) {
        ui.label("Frequency response app currently not implemented.");
    }

    fn get_label(&self) -> &str {
        &self.label
    }
}


// impl eframe::App for TemplateApp {
//     fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
//         let Self { label, value } = self;

//         egui::SidePanel::left("side_panel").show(ctx, |ui| {
//             ui.heading("Side Panel");

//             ui.horizontal(|ui| {
//                 ui.label("Write something: ");
//                 ui.text_edit_singleline(label);
//             });

//             ui.add(egui::Slider::new(value, 0.0..=10.0).text("value"));
//             if ui.button("Increment:").clicked() {
//                 *value += 1.0;
//             }

//             ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
//                 ui.horizontal(|ui| {
//                     ui.spacing_mut().item_spacing.x = 0.0;
//                     ui.label("powered by ");
//                     ui.hyperlink_to("egui", "https://github.com/emilk/egui");
//                     ui.label(" and ");
//                     ui.hyperlink_to(
//                         "eframe",
//                         "https://github.com/emilk/egui/tree/master/crates/eframe",
//                         );
//                     ui.label(".");
//                 });
//             });
//         });

//     }
// }
