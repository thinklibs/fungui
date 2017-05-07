//! # Design Goals
//!
//! - Fast
//! - Accessible for most users and devices
//! - CSS like features
//! - Simple API
//! - Cross-platform/renderer
//! - Dynamic (Being able to switch devices/accessibility features on the fly)
//! - Events
//! - Editor
//!
//! ## Bonus goals
//!
//! - C API
//!
//! # Problems
//!
//! - How will elements be represented in rust? (No inheritence)
//! - Support for custom elements or just base ones?
//! - Language/format for UI description
//! - Language/format for styling
//! - Templates?
//!
//!
//! # Rendering
//!
//! - Mainly hardware rendering, software rendering
//!   for certain complex effects
//! - Render to texture and attempt to not redraw
//!   often
//!   * How do we render these textures efficently? Might take multiple draws
//!     every frame to render the ui
//!   * Most likely not a bottleneck though
//!   * Render to a texture array? Lots of wasted memory
//!      - Unless an atlas is used, is this possible?
//! - Seperate from core crate so that its not tied to a single api