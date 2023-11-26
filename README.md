# eventify
eventify is a helper library for triggering events in your application.

# Examples
### Events
```rust
use eventify::event::*;

fn main() {
    let example_event = Event::new();
    let event_hook = example_event.hook(move || {
        println!("Hello World!");
    });
    example_event.invoke(&());
}
```
### Properties
```rust
use eventify::property::*;

fn main() {
    let example_property = Property::new(0);
    let property_hook = example_property.hook(move |value| {
        println!("Value: {}", value);
    });
    example_property.set(1);
}
```   
}