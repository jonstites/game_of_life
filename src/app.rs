use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};
use yew::{html, Callback, MouseEvent, Component, ComponentLink, Html, ShouldRender, NodeRef};
use yew::services::{IntervalService, RenderService, Task};
use wasm_bindgen::JsValue;
use wasm_bindgen::JsCast;
//use rand::Rng;
use std::time::Duration;
extern crate js_sys;



pub struct App {
    canvas: Option<HtmlCanvasElement>,
    node_ref: NodeRef,
    render_loop: Option<Box<dyn Task>>,
    link: ComponentLink<Self>,
    timer: Box<dyn Task>,
    active: bool,
}

pub enum Msg {
    Tick,
    Noop,
}

impl Component for App {
    type Message = Msg;
    type Properties = ();
    
    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        let mut interval = IntervalService::new();
        let handle = interval.spawn(Duration::from_millis(200), link.callback(|_| Msg::Tick));

        App {
            canvas: None,
            link: link,
            node_ref: NodeRef::default(),
            render_loop: None,
            timer: Box::new(handle),
            active: false,
        }
    }

    fn mounted(&mut self) -> ShouldRender {
        // Once mounted, store references for the canvas and GL context. These can be used for
        // resizing the rendering area when the window or canvas element are resized, as well as
        // for making GL calls.

        let canvas = self.node_ref.cast::<HtmlCanvasElement>().unwrap();

        self.canvas = Some(canvas);
        let render_frame = self.link.callback(|_| Msg::Noop);
        let handle = RenderService::new().request_animation_frame(render_frame);

        // A reference to the handle must be stored, otherwise it is dropped and the render won't
        // occur.
        self.render_loop = Some(Box::new(handle));

        false
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::Tick => {
                self.render_canvas();
                false
            },
            Msg::Noop => {
                false
            }
        }
    }

    fn view(&self) -> Html {
        
        html! {
            <body>
                <div>
                    <canvas ref={self.node_ref.clone()} />
                </div>
            </body>
        }
    }
}

impl App {

    fn render_canvas(&mut self) {
        self.canvas.as_ref().unwrap().set_width(500);
        self.canvas.as_ref().unwrap().set_height(500);

        let ctx = self.canvas.as_ref()
            .expect("Canvas not loaded")
            .get_context("2d")
            .expect("Can't get 2d canvas.")
            .unwrap()
            .dyn_into::<CanvasRenderingContext2d>()
            .unwrap();
                
        
        ctx.rect(10.0, 10.0, 150.0, 100.0);
        if /*rand::thread_rng().gen()*/ js_sys::Math::random() < 0.5 {
            ctx.set_fill_style(&JsValue::from_str("red"));
        }
        ctx.fill();
        let render_frame = self.link.callback(|_| Msg::Noop);
        let handle = RenderService::new().request_animation_frame(render_frame);
        self.render_loop = Some(Box::new(handle));
    }
}