use slint::slint;

slint!{
    import { Button, VerticalBox } from "std-widgets.slint";
    export component App inherits Window {
        in property<int> counter: 1;
        callback clicked <=> btn.clicked;
        VerticalBox {
            Text {
                text: "Hello, world, " + counter;
            }
            btn := Button {
                text: "Click me";
            }
        }
    }
}


pub fn run_app() {
    let app = App::new().unwrap();
    let weak = app.as_weak();
    app.on_clicked(move || {
        let app: App = weak.upgrade().unwrap();
        app.set_counter(app.get_counter() + 1);
    });
    app.run().unwrap();
}