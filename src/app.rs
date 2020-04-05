use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};
use yew::{html, Callback, MouseEvent, Component, ComponentLink, Html, ShouldRender, NodeRef};
use yew::services::{RenderService, Task};

use wasm_bindgen::JsCast;

pub struct App {
    canvas: Option<HtmlCanvasElement>,
    node_ref: NodeRef,
    render_loop: Option<Box<dyn Task>>,
    link: ComponentLink<Self>,
    clicked: bool,
    onclick: Callback<MouseEvent>,
}

pub enum Msg {
    Click,
    Render,
}

impl Component for App {
    type Message = Msg;
    type Properties = ();
    
    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        App {
            canvas: None,
            clicked: false,
            onclick: link.callback(|_| Msg::Click),
            link: link,
            node_ref: NodeRef::default(),
            render_loop: None,
        }
    }

    fn mounted(&mut self) -> ShouldRender {
        // Once mounted, store references for the canvas and GL context. These can be used for
        // resizing the rendering area when the window or canvas element are resized, as well as
        // for making GL calls.

        let canvas = self.node_ref.cast::<HtmlCanvasElement>().unwrap();

        self.canvas = Some(canvas);
        let render_frame = self.link.callback(|_| Msg::Render);
        let handle = RenderService::new().request_animation_frame(render_frame);

        // A reference to the handle must be stored, otherwise it is dropped and the render won't
        // occur.
        self.render_loop = Some(Box::new(handle));

        false
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::Click => {
                self.clicked = true;
                true // Indicate that the Component should re-render
            },
            Msg::Render => {
                self.render_canvas();
                false
            }
        }
    }

    fn view(&self) -> Html {
        let button_text = if self.clicked { "Clicked!" } else { "Click me!" };
        
        html! {
            <body>
            <div>
                <button onclick=&self.onclick>{ button_text }</button>
                </div>
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
        ctx.fill();
    }
}