//! Aggregator for the `Renderer/*` spec mirrors (one submodule per ported
//! Jasmine spec under `packages/engine/Specs/Renderer`).

#[path = "renderer/renderer_shader_destination_spec.rs"]
mod renderer_shader_destination_spec;
#[path = "renderer/renderer_pass_state_spec.rs"]
mod renderer_pass_state_spec;
#[path = "renderer/renderer_clear_command_spec.rs"]
mod renderer_clear_command_spec;
#[path = "renderer/renderer_draw_command_spec.rs"]
mod renderer_draw_command_spec;
#[path = "renderer/renderer_pass_spec.rs"]
mod renderer_pass_spec;
