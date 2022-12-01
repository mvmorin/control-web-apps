#![warn(clippy::all, rust_2018_idioms)]

#[allow(unused_imports)]
use basic_print::basic_print; // basic print for print-debugging

use pole_position::PolePos;
use frequency_response::FreqResp;

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
            ui.centered_and_justified(|ui| {
                match self.cur_app_idx {
                    None => {
                        ui.label("Select an application in the bar above.");
                    },
                    Some(idx) => {self.apps[idx].draw_app(ui);},
                };

            });
        });
    }
}


mod pole_position {
    #[allow(unused_imports)]
    use basic_print::basic_print; // basic print for print-debugging

    use egui::plot::{ Line, LineStyle, Plot, PlotPoint, PlotPoints, PlotUi, Points, MarkerShape, };
    use egui::{ Ui, Color32, Vec2, Layout, Align, InnerResponse, Response, };

    use crate::LinearSystems::*;
    use crate::CentralApp;

    use std::f64::consts::PI;
    use std::ops::Range;
    use std::rc::Rc;


    #[derive(PartialEq,Debug,Clone,Copy)]
    enum Order {
        First,
        Second,
    }

    #[derive(PartialEq,Debug,Clone,Copy)]
    enum Display {
        StepResponse,
        BodeDiagram,
    }

    #[derive(Debug)]
    pub struct PolePos{
        label: String,

        order: Order,
        display: Display,

        fo: FirstOrderSystem,
        so: SecondOrderSystem,

        pole_drag_offset: Option<(f64,f64)>,
    }



    impl PolePos {
        pub fn new(label: String) -> PolePos {
            PolePos{
                label,
                order: Order::First,
                display: Display::StepResponse,
                fo: FirstOrderSystem{T: 1.0},
                so: SecondOrderSystem{d: 0.5, w: 0.75},
                pole_drag_offset: None,
            }
        }


        fn pole_plot(&mut self, ui: &mut Ui, width: f32, height: f32) {
            // Plot params
            let cross_radius = 10.0;
            let re_bounds = -3.0..1.0;
            let im_bounds = -1.0..1.0;


            // Plot data
            let p = match self.order {
                Order::First => {self.fo.poles()},
                Order::Second => {self.so.poles()},
            };

            let data = Points::new(p);
            let unit_circle = Line::new(PlotPoints::from_parametric_callback(|t| (t.sin(), t.cos()), 0.0..(2.0*PI), 100));

            // Plot
            let resp = plot_show(
                ui, "Pole Placement",
                width, height, re_bounds, im_bounds,
                |plot| {plot.data_aspect(1.0)},
                |plot_ui| {
                    plot_ui.line( unit_circle.color(Color32::GRAY) );
                    plot_ui.points( data.shape(MarkerShape::Cross).color(Color32::BLACK).radius(cross_radius) );
                    plot_ui.pointer_coordinate()
                });
            let InnerResponse{ response: _, inner: (show_response, pointer) } = resp;

            // Handle dragging
            if show_response.dragged() {
                let (re_off, im_off) = match self.pole_drag_offset {
                    Some((x,y)) => (x,y),
                    None => {
                        // We have just started to drag, find offset to closest pole so we don't jump
                        //
                        // For now, just assume we want to place the pole where we click
                        self.pole_drag_offset = Some((0.0, 0.0));
                        (0.0,0.0)
                    },
                };


                if let Some(PlotPoint{x:re,y:im}) = pointer { // This should never fail
                    let (re, im) = (re+re_off, im+im_off);

                    match self.order {
                        Order::First => self.fo.adjust_poles_to(re,im),
                        Order::Second => self.so.adjust_poles_to(re,im),
                    };
                }
            } else {
                self.pole_drag_offset = None;
            }
        }

        fn step_response_plot(&mut self, ui: &mut Ui, width: f32, height: f32) {
            // Plot params
            let n_samples = 100;
            let t_end = 10.0;
            let pad_ratio = 0.1;

            // Calculate plot bounds
            let t_bounds = (0.0 - t_end*pad_ratio)..(t_end + t_end*pad_ratio);
            let y_bounds = (0.0-pad_ratio)..(1.0+pad_ratio);

            // Plot data
            let sys: Box<dyn LinearSystem> = match self.order {
                Order::First => Box::new(self.fo),
                Order::Second => Box::new(self.so),
            };
            let data = Line::new(PlotPoints::from_explicit_callback(move |t| sys.step_response(t), t_bounds.clone(), n_samples));

            // Plot
            plot_show(
                ui, "Step Response",
                width, height, t_bounds, y_bounds,
                |plot| {plot},
                |plot_ui| {
                    plot_ui.line( data.color(Color32::RED).style(LineStyle::Solid) );
                });
        }

        fn bode_plot(&mut self, ui: &mut Ui, width: f32, height: f32) {
            // Plot params
            let n_samples = 100;
            let w_bounds_exp = -4.0..2.0;

            // Plot data
            let sys: Rc<dyn LinearSystem> = match self.order {
                Order::First => Rc::new(self.fo),
                Order::Second => Rc::new(self.so),
            };
            let sys_cb = sys.clone();
            let amp_data = Line::new(PlotPoints::from_explicit_callback(move |we| sys_cb.bode_amplitude(10f64.powf(we)).log10(), w_bounds_exp.clone(), n_samples));

            let sys_cb = sys.clone();
            let phase_data = Line::new(PlotPoints::from_explicit_callback(move |we| sys_cb.bode_phase(10f64.powf(we)), w_bounds_exp.clone(), n_samples));

            // Plot
            ui.allocate_ui_with_layout(
                Vec2{x: width, y: height},
                Layout::top_down(Align::LEFT),
                |ui| {
                    let height = (height - ui.spacing().item_spacing.y)/2.0;
                    plot_show(
                        ui, "Bode Plote - Amplitude", width, height, w_bounds_exp.clone(), -4.0..5f64.log10(),
                        |plot| {plot},
                        |plot_ui| {
                            plot_ui.line( amp_data.color(Color32::RED).style(LineStyle::Solid) );
                        });
                    plot_show(
                        ui, "Bode Plote - Phase", width, height, w_bounds_exp.clone(), -PI/2.0..PI/2.0,
                        |plot| {plot},
                        |plot_ui| {
                            plot_ui.line( phase_data.color(Color32::RED).style(LineStyle::Solid) );
                        });
                });
        }



        fn order_selection(&mut self, ui: &mut Ui) {
            ui.heading("Select System Order");
            ui.horizontal(|ui| {
                ui.radio_value(&mut self.order, Order::First, "First order");
                ui.radio_value(&mut self.order, Order::Second, "Second order");
            });
        }

        fn display_selection(&mut self, ui: &mut Ui) {
            ui.heading("Select Display");
            ui.horizontal(|ui| {
                ui.radio_value(&mut self.display, Display::StepResponse, "Step Response");
                ui.radio_value(&mut self.display, Display::BodeDiagram, "Bode Diagram");
            });
        }

        fn parameter_sliders(&mut self, ui: &mut Ui) {
            match self.order {
                Order::First => {
                    ui.heading("G(s) = 1/(sT - 1)");
                    ui.add(egui::Slider::new(&mut self.fo.T, 0.2..=100.0)
                           .text("T")
                           .logarithmic(true)
                          );
                },
                Order::Second => {
                    ui.heading("G(s) = ω^2/(s^2 + 2δωs+ ω^2)");
                    ui.add(egui::Slider::new(&mut self.so.d, 0.0..=1.5).text("δ"));
                    ui.add(egui::Slider::new(&mut self.so.w, 0.0..=2.0).text("ω"));
                },
            };
        }
    }


    impl CentralApp for PolePos {
        fn draw_app(&mut self, ui: &mut Ui) {
            // ui.spacing_mut().item_spacing.x = 10.0;

            let max_width = 550.0;
            let Vec2{x,y} = ui.available_size();
            let is_vertical = x < max_width;


            if is_vertical {

                egui::Grid::new("app_grid")
                    .num_columns(1)
                    .show(ui, |ui| {

                        ui.vertical(|ui| {
                            self.order_selection(ui);
                            ui.separator();
                            self.display_selection(ui);
                            ui.separator();
                            self.parameter_sliders(ui);
                            ui.separator();
                        });
                        ui.end_row();

                        let Vec2{x,y} = ui.available_size();
                        let mut width = x;
                        let mut height = y/2.0;

                        if width >= height*1.75 {
                            width = height*1.75
                        } else {
                            height = width/1.75
                        }

                        self.pole_plot(ui, width, height);
                        ui.end_row();

                        match self.display {
                            Display::StepResponse => self.step_response_plot(ui, width, height),
                            Display::BodeDiagram => self.bode_plot(ui, width, height),
                        };
                    });

            } else {
                let mut width = (x/2.0).min(max_width);
                let mut height = y/2.0;

                if width >= height*1.75 {
                    width = height*1.75
                } else {
                    height = width/1.75
                }

                egui::Grid::new("app_grid")
                    .num_columns(2)
                    .show(ui, |ui| {

                        ui.vertical(|ui| {
                            self.order_selection(ui);
                            ui.add_space(20.0);
                            self.parameter_sliders(ui);
                        });

                        self.pole_plot(ui, width, height);

                        ui.end_row();
                        self.step_response_plot(ui, width, height);
                        self.bode_plot(ui, width, height);
                    });
            }


        }

        fn get_label(&self) -> &str {
            &self.label
        }
    }


    fn plot_show<R>(
        ui: &mut Ui, title: &str,
        width: f32, height: f32, x_bounds: Range<f64>, y_bounds: Range<f64>,
        plot_mod_fn: impl FnOnce(Plot) -> Plot,
        build_fn: impl FnOnce(&mut PlotUi) -> R,
        ) -> InnerResponse<(Response,R)>
    {
        ui.allocate_ui_with_layout(
            Vec2{x: width, y: height},
            Layout::top_down(Align::LEFT),
            |ui| {
                ui.heading(title);

                let mut plot =
                    Plot::new(title)
                    .allow_scroll(false)
                    .allow_zoom(false)
                    .allow_boxed_zoom(false)
                    .allow_drag(false)
                    .show_x(false)
                    .show_y(false)
                    .include_x(x_bounds.start)
                    .include_x(x_bounds.end)
                    .include_y(y_bounds.start)
                    .include_y(y_bounds.end)
                    .set_margin_fraction(Vec2{x:0.0, y:0.0})
                    ;

                plot = plot_mod_fn(plot);

                let InnerResponse{response: show_response, inner: build_resp} = plot.show(ui, build_fn );
                (show_response, build_resp)
            })
    }

}





pub mod LinearSystems {
    #![allow(non_snake_case)]

    pub trait LinearSystem {
        fn step_response(&self, t: f64) -> f64;
        fn bode_amplitude(&self, w: f64) -> f64;
        fn bode_phase(&self, w: f64) -> f64;
        fn poles(&self) -> Vec<[f64; 2]>;
        fn adjust_poles_to(&mut self, re: f64, im: f64);
    }

    #[derive(Debug,Clone,Copy)]
    pub struct FirstOrderSystem {
        // first order system 1/(sT + 1)
        // pole = -1/T
        // https://www.tutorialspoint.com/control_systems/control_systems_response_first_order.htm
         pub T: f64,
    }

    impl LinearSystem for FirstOrderSystem {
        fn poles(&self) -> Vec<[f64; 2]> {
            vec![[-1.0/self.T, 0.0]]
        }

        fn step_response(&self, t: f64) -> f64 {
            if t >= 0.0 {
                1.0 - (-t/self.T).exp()
            } else {
                0.0
            }
        }

        fn bode_amplitude(&self, w: f64) -> f64 {
            1.0 / ( ((w*self.T).powi(2) + 1.0).sqrt() )
        }

        fn bode_phase(&self, w: f64) -> f64 {
            -(w*self.T).atan()
        }

        fn adjust_poles_to(&mut self, re: f64, _im: f64) {
            let pole_bound = -0.01;


            if re >= pole_bound {
                self.T = -1.0/pole_bound;
            } else {
                self.T = -1.0/re;
            }
        }
    }

    #[derive(Debug,Clone,Copy)]
    pub struct SecondOrderSystem { // TODO fix this, the step response is not corerct
        // second order system w^2/(s^2 + 2dw s + w^2)
        // poles = -dw +- w sqrt(d^2 - 1)
        // https://www.tutorialspoint.com/control_systems/control_systems_response_second_order.htm
        pub d: f64,
        pub w: f64,
    }

    impl LinearSystem for SecondOrderSystem {
        fn poles(&self) -> Vec<[f64; 2]> {
            let (d,w) = (self.d, self.w);

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

        fn step_response(&self, t: f64) -> f64 {
            let (d,w) = (self.d, self.w);

            if t < 0.0 {
                return 0.0
            }

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
        }

        fn bode_amplitude(&self, _w: f64) -> f64 {
            1.0
        }

        fn bode_phase(&self, _w: f64) -> f64 {
            0.0
        }

        fn adjust_poles_to(&mut self, _re: f64, _im: f64) {
        }
    }
}
















mod frequency_response {
    use crate::CentralApp;

    pub struct FreqResp{
        label: String,
    }

    impl FreqResp {
        pub fn new(label: String) -> FreqResp {
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
}
