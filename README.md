# Bevy Simple State Machine

[![Crates.io](https://img.shields.io/crates/v/bevy-simple-state-machine)](https://crates.io/crates/bevy-simple-state-machine)
[![docs](https://docs.rs/bevy-simple-state-machine/badge.svg)](https://docs.rs/bevy-simple-state-machine/)
[![MIT/Apache 2.0](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](./LICENSE)

Plugin for the [Bevy Engine](https://bevyengine.org) which implements
a rudimentary animation state machine system.

To use this, you have to add the `SimpleStateMachinePlugin` to you app:

```rust
App::new()
    .add_plugins(DefaultPlugins)
    .add_plugin(SimpleStateMachinePlugin::new());
```

And then insert an `AnimationStateMachine` component on your entities:

```rust
fn setup(mut commands: Commands) {
    let starting_state = "idle";
    let my_states_map = HashMap::from([
        ("idle".to_string(), AnimationState{
            name: "idle".to_string(),
            clip: idle_clip_handle,
            interruptible: true,
        }),
        ("run".to_string(), AnimationState{
            name: "run".to_string(),
            clip: run_clip_handle,
            interruptible: true,
        }),
    ]);
    let my_states_transitions_vec = vec![
        StateMachineTransition {
        start_state: AnimationStateRef::from_string("idle"),
        end_state: AnimationStateRef::from_string("run"),
        trigger: StateMachineTrigger::from(|vars| vars["run"].is_bool(true)),
    }];
    let state_machine_vars = HashMap::from([
        ("run".to_string(), StateMachineVariableType::Bool(false)),    
    ]);
     
    commands.spawn_bundle(SpatialBundle::default())
        .insert(AnimationPlayer::default())
        .insert(AnimationStateMachine::new(
            starting_state,
            my_states_map,
            my_states_transitions_vec,
            state_machine_vars,
        ));
}
```

---

## Currently supported features:

 - Custom transition conditions
 - Transitions from wildcard state AnyState
 - Events emitted on transition end

Currently, transitions end on the same frame they are triggered.

Animation blending and transition duration are not implemented.

---
## Bevy Compatibility:

| Bevy Version | Plugin Version       |
|--------------|----------------------|
| `0.8`        | `main`               |
| `0.8`        | `0.1.0`              |