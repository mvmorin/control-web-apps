#![warn(clippy::all, rust_2018_idioms)]
#![allow(non_snake_case)]

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
            Box::new(PolePos::new("Pole Positioning".to_string() )),
            Box::new(FreqResp::new("Frequency Response".to_string() )),
        ];

        // ControlApp{cur_app_idx: None, apps }
        ControlApp{cur_app_idx: Some(0), apps }
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


            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
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
            match self.cur_app_idx {
                None => {
                    ui.centered_and_justified(|ui| {
                        ui.label("Select an application in the bar above.");
                    });
                },
                Some(idx) => {self.apps[idx].draw_app(ui);},
            };
        });
    }
}



use egui::plot::{
    Line,
    LineStyle,
    Plot,
    PlotPoints,
    Points,
    MarkerShape,
};
use egui::{
    Color32,
    Vec2,
    Layout,
    InnerResponse,
};
use std::f64::consts::PI;


struct PolePos{
    label: String,

    // first order system 1/(sT + 1)
    // pole = -1/T
    // https://www.tutorialspoint.com/control_systems/control_systems_response_first_order.htm
    T: f64,

    // second order system w^2/(s^2 + 2dw s + w^2)
    // poles = -dw +- w sqrt(d^2 - 1)
    // https://www.tutorialspoint.com/control_systems/control_systems_response_second_order.htm
    d: f64,
    w: f64,

    order: Order,
}

#[allow(dead_code)]
#[derive(PartialEq)]
enum Order {
    First,
    Second,
}

impl PolePos {
    fn new(label: String) -> PolePos {
        PolePos{
            label,
            T: 1.0,
            d: 0.5,
            w: 1.0,
            order: Order::First,
        }
    }

    fn step_resp_line(&self, t_end: f64, n_steps: usize) -> Line {
        let (T,d,w) = (self.T, self.d, self.w);

        let f_first = move |t: f64| 1.0 - (-t/T).exp();

        let f_second = move |t: f64| {
            if d == 0.0 {
                1.0 - (w*t).cos()
            } else if (0.0 < d) && (d < 1.0) {
                let d_1_sqrt = (1.0 - d.powi(2)).sqrt();
                let th = d_1_sqrt.asin();
                1.0 - ( (-w*t).exp() )*( (w*t + th).sin() )/ d_1_sqrt
            } else if d == 1.0 {
                1.0 - ( (-w*t).exp() )*(1.0 + w*t)
            } else {
                let d_1_sqrt = (d.powi(2)-1.0).sqrt();
                1.0
                    + (-t*w*(d + d_1_sqrt)).exp() / (2.0*(d+d_1_sqrt)*d_1_sqrt)
                    - (-t*w*(d - d_1_sqrt)).exp() / (2.0*(d-d_1_sqrt)*d_1_sqrt)
            }
        };

        match self.order {
            Order::First => Line::new(PlotPoints::from_explicit_callback(f_first, 0.0..t_end, n_steps)),
            Order::Second => Line::new(PlotPoints::from_explicit_callback(f_second, 0.0..t_end, n_steps)),
        }
    }

    // fn bode_amp_line(&self, w_start: f64, w_end: f64, n_steps: usize) -> Line {
    //     Line::new(PlotPoints::from_explicit_callback(|_w| 1.0, w_start..w_end, n_steps))
    // }

    // fn bode_phase_line(&self, w_start: f64, w_end: f64, n_steps: usize) -> Line {
    //     Line::new(PlotPoints::from_explicit_callback(|_w| 0.0, w_start..w_end, n_steps))
    // }

    fn poles(&self) -> Vec<[f64;2]> {
        let (T,d,w) = (self.T, self.d, self.w);

        match self.order {
            Order::First => vec![[-1.0/T, 0.0]],
            Order::Second => {
                if d == 0.0 {
                    vec![
                        [0.0, w],
                        [0.0, -w],
                    ]
                } else if (0.0 < d) && (d < 1.0) {
                    vec![
                        [-d*w, (1.0-d.powi(2)).sqrt()*w],
                        [-d*w, -(1.0-d.powi(2)).sqrt()*w],
                    ]
                } else if d == 1.0 {
                    vec![
                        [-w, 0.0],
                        [-w, 0.0],
                    ]
                } else {
                    vec![
                        [-d*w + (d.powi(2) -1.0).sqrt()*w, 0.0],
                        [-d*w - (d.powi(2) -1.0).sqrt()*w, 0.0],
                    ]
                }
            }
        }
    }

    fn pole_markers(&self) -> Points {
        Points::new(self.poles())
            .shape(MarkerShape::Cross)
    }

    // fn create_dummy_plot(&mut self, ui: &mut egui::Ui) {
    //     let sr_line =
    //         self.step_resp_line(10.0, 100)
    //         .color(Color32::from_rgb(200, 100, 100))
    //         .style(LineStyle::Solid)
    //         .name("Step Response");
    //     let sr_plot =
    //         fixed_plot("step_response")
    //         .set_margin_fraction(Vec2::new(0.05,0.05)) ;
    //     sr_plot.show(ui, |plot_ui| plot_ui.line(sr_line) );
    // }
}


impl CentralApp for PolePos {
    fn draw_app(&mut self, ui: &mut egui::Ui) {
        let Vec2{x,y} = ui.available_size();

        ui.heading("Select System Order");
        ui.radio_value(&mut self.order, Order::First, "First order");
        ui.radio_value(&mut self.order, Order::Second, "Second order");

        ui.separator();
        match self.order {
            Order::First => {
                ui.heading("G(s) = 1/(sT - 1)");
                ui.add(egui::Slider::new(&mut self.T, 0.2..=10.0)
                    .text("T")
                    .logarithmic(true)
                    );
                ui.add_visible(false, egui::Slider::new(&mut self.T, 0.2..=10.0));
            },
            Order::Second => {
                ui.heading("G(s) = ω^2/(s^2 + 2δωs+ ω^2)");
                ui.add(egui::Slider::new(&mut self.d, 0.0..=1.5).text("δ"));
                ui.add(egui::Slider::new(&mut self.w, 0.0..=2.0).text("ω"));
            },
        };
        ui.separator();

        ui.separator();
        let mut plot = fixed_plot("pole_plot", y/2.0, y/2.0);

        let InnerResponse {
            response,
            inner: drag_delta
        } = plot.show(ui,|plot_ui| {
            plot_ui.line(
                unit_circle()
                .color(Color32::GRAY)
                );
            plot_ui.points(
                self.pole_markers()
                .color(Color32::BLACK)
                .radius(10.0)
                );
            plot_ui.pointer_coordinate_drag_delta()
        });

        ui.label(format!("{:?}",drag_delta));

    }

    fn get_label(&self) -> &str {
        &self.label
    }
}


fn fixed_plot(id: &str, width: f32, height: f32) -> Plot {
    Plot::new(id)
        .allow_scroll(false)
        .allow_zoom(false)
        .allow_boxed_zoom(false)
        .allow_drag(false)
        .show_x(false)
        .show_y(false)
        .width(width)
        .height(height)
}

fn unit_circle() -> Line {
    Line::new(PlotPoints::from_parametric_callback(|t| (t.sin(), t.cos()), 0.0..(2.0*PI), 100))
}







struct FreqResp{
    label: String,
}

impl FreqResp {
    fn new(label: String) -> FreqResp {
        FreqResp{
            label,
        }
    }
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

//             ui.with_layout(Layout::bottom_up(egui::Align::LEFT), |ui| {
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
