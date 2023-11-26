# eventify
A utility library for variety of event-based programming patterns.

*** Eventify is still in early development, and is not yet ready for use in production! ***

# Examples
### Events
```rust
use eventify::event::*;

fn main() {
    let event = Event::new();
    
    let hook = event.hook(|_| {
        println!("Event fired!");
    });
    
    event.emit(&());
}
```
### Properties
```rust
use eventify::property::*;

fn main() {
    let property = Property::new("".to_string());

    let binding = property.bind(|value| {
        println!("Property changed to: {}", value);
    });

    *property.write().unwrap() = "Hello, world!".to_string();
}
```