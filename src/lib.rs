#![warn(missing_docs)]
//! # Bevy Simple State Machine
//!
//! Plugin for the [Bevy Engine](https://bevyengine.org) which implements
//! a rudimentary animation state machine system.
//!
//! To use this, you have to add the `SimpleStateMachinePlugin` to you app
//!
//! ```
//! # use bevy::prelude::*;
//! # use bevy_simple_state_machine::SimpleStateMachinePlugin;
//! App::new()
//!     .add_plugins(DefaultPlugins)
//!     .add_plugin(SimpleStateMachinePlugin::new());
//! ```
//!
//! And then insert an `AnimationStateMachine` component on your entities
//!
//! ```
//! # use bevy_simple_state_machine::*;
//! # use bevy::{prelude::*, utils::HashMap};
//! fn setup(mut commands: Commands) {
//! # let idle_clip_handle: Handle<AnimationClip> = Handle::default();
//! # let run_clip_handle: Handle<AnimationClip> = Handle::default();
//!     let starting_state = "idle";
//!     let my_states_map = HashMap::from([
//!         ("idle", AnimationState{
//!             name: "idle".to_string(),
//!             clip: idle_clip_handle,
//!             interruptible: true,
//!         }),
//!         ("run", AnimationState{
//!             name: "run".to_string(),
//!             clip: run_clip_handle,
//!             interruptible: true,
//!         }),
//!     ]);
//!     let my_states_transitions_vec = vec![
//!         StateMachineTransition {
//!         start_state: AnimationStateRef::from_string("idle"),
//!         end_state: AnimationStateRef::from_string("run"),
//!         trigger: StateMachineTrigger::from(|vars| vars["run"].is_bool(true)),
//!     }];
//!     let state_machine_vars = HashMap::from([
//!         ("run", StateMachineVariableType::Bool(false)),    
//!     ]);
//!      
//!     commands.spawn_bundle(SpatialBundle::default())
//!         .insert(AnimationPlayer::default())
//!         .insert(AnimationStateMachine::new(
//!             starting_state,
//!             my_states_map,
//!             my_states_transitions_vec,
//!             state_machine_vars,
//!         ));
//! }
//! ```
//!
//! And then you can control it changing the values of the state
//! machine variables
//!
//! ```
//! # use bevy_simple_state_machine::*;
//! # use bevy::{prelude::*, utils::HashMap};
//! # let mut state_machine = AnimationStateMachine::new(
//! #   "idle",
//! #   HashMap::default(),
//! #   vec![],
//! #   HashMap::from([("run", StateMachineVariableType::Bool(false))]),
//! # );
//! state_machine.update_variable("run", StateMachineVariableType::Bool(true));
//! ```
//!
//! ## Currently supported features:
//!
//!  - Custom transition conditions
//!  - Transitions from wildcard state AnyState
//!  - Events emitted on transition end
//!  - Internal state machine variables
//!
//! Currently, transitions end on the same frame they are triggered.
//!
//! Animation blending and transition duration are not implemented.

use std::{
    fmt::{Debug, Display},
    sync::Arc,
};

use bevy::{prelude::*, reflect::FromReflect, utils::HashMap};

/// Plugin that handles all state machine executions
///
/// Include this in your app to enable this crate
/// ```
/// # use bevy::prelude::*;
/// # use bevy_simple_state_machine::SimpleStateMachinePlugin;
/// App::new()
///     .add_plugins(DefaultPlugins)
///     .add_plugin(SimpleStateMachinePlugin::new());
/// ```
#[derive(Default)]
pub struct SimpleStateMachinePlugin {}

impl Plugin for SimpleStateMachinePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<TransitionEndedEvent>()
            .register_type::<AnimationStateMachine>()
            .register_type::<AnimationStateRef>()
            .register_type::<AnimationState>()
            .register_type::<StateMachineVariableType>()
            .register_type::<StateMachineTransition>()
            .add_system(Self::check_transitions.label(StateMachineSystemLabel::StateMachineLabel))
            .add_system(
                Self::init_state_machines.label(StateMachineSystemLabel::StateMachineLabel),
            );
    }
}

impl SimpleStateMachinePlugin {
    /// Creates a new instance of [`SimpleStateMachinePlugin`]    
    pub fn new() -> Self {
        Self::default()
    }

    fn check_transitions(
        mut state_machines_query: Query<(Entity, &mut AnimationStateMachine, &mut AnimationPlayer)>,
        animations: Res<Assets<AnimationClip>>,
        mut event_writer: EventWriter<TransitionEndedEvent>,
    ) {
        for (entity, mut state_machine, mut player) in &mut state_machines_query {
            if let Some(current_state) = state_machine.current_state() {
                if current_state.interruptible
                    || AnimationStateMachine::animation_finished(
                        player.as_mut(),
                        &current_state,
                        animations.as_ref(),
                    )
                {
                    for transition in state_machine.transitions_from_current_state() {
                        if transition.trigger.evaluate(&state_machine.variables) {
                            if let Some(next_state) =
                                state_machine.get_state(transition.end_state.unwrap())
                            {
                                debug!("triggering {}", transition);
                                state_machine.current_state = next_state.name;
                                player.play(next_state.clip);
                                event_writer.send(TransitionEndedEvent {
                                    entity,
                                    origin: current_state.state_ref(),
                                    end: transition.end_state,
                                })
                            }
                        }
                    }
                }
            }
        }
    }

    fn init_state_machines(
        mut state_machines_query: Query<
            (&AnimationStateMachine, &mut AnimationPlayer),
            Added<AnimationStateMachine>,
        >,
    ) {
        for (state_machine, mut player) in &mut state_machines_query {
            if let Some(current_state) = state_machine.current_state() {
                player.play(current_state.clip);
            }
        }
    }
}

/// State machine system label
///
/// You can use this if you need a specific order for your systems
#[derive(SystemLabel, Clone)]
pub enum StateMachineSystemLabel {
    #[allow(missing_docs)]
    StateMachineLabel,
}

/// Internal state machine variables map type
pub type StateMachineVariables = HashMap<String, StateMachineVariableType>;

/// State machine variable type
#[derive(Clone, Reflect, FromReflect, PartialEq)]
pub enum StateMachineVariableType {
    /// Stores a bool
    Bool(bool),
    /// Stores an f32
    F32(f32),
    /// Stores an i32
    I32(i32),
    /// Stores an u32
    U32(u32),
    /// Stores a String
    String(String),
}

impl StateMachineVariableType {
    /// Tests if the variable is equal to the given value
    pub fn is_bool(&self, value: bool) -> bool {
        *self == Self::Bool(value)
    }

    /// Tests if the variable is equal to the given value
    pub fn is_i32(&self, value: i32) -> bool {
        *self == Self::I32(value)
    }

    /// Tests if the variable is equal to the given value
    pub fn is_u32(&self, value: u32) -> bool {
        *self == Self::U32(value)
    }

    /// Tests if the variable is equal to the given value
    pub fn is_f32(&self, value: f32) -> bool {
        *self == Self::F32(value)
    }
}

/// Main state machine component
///
/// Insert this on the entity you want to control with the state machine.
///
/// ## Note:
/// To function, the component requires an [`AnimationPlayer`] on the same entity.
///
/// ---
///
/// Example
/// ```
/// # use bevy_simple_state_machine::*;
/// # use bevy::{prelude::*, utils::HashMap};
/// fn setup(mut commands: Commands) {
/// # let idle_clip_handle: Handle<AnimationClip> = Handle::default();
/// # let run_clip_handle: Handle<AnimationClip> = Handle::default();
///     let starting_state = "idle";
///     let my_states_map = HashMap::from([
///         ("idle", AnimationState{
///             name: "idle".to_string(),
///             clip: idle_clip_handle,
///             interruptible: true,
///         }),
///         ("run", AnimationState{
///             name: "run".to_string(),
///             clip: run_clip_handle,
///             interruptible: true,
///         }),
///     ]);
///     let my_states_transitions_vec = vec![
///         StateMachineTransition {
///         start_state: AnimationStateRef::from_string("idle"),
///         end_state: AnimationStateRef::from_string("run"),
///         trigger: StateMachineTrigger::from(|vars| vars["run"].is_bool(true)),
///     }];
///     let state_machine_vars = HashMap::from([
///         ("run", StateMachineVariableType::Bool(false)),
///     ]);
///      
///     commands.spawn_bundle(SpatialBundle::default())
///         .insert(AnimationPlayer::default())
///         .insert(AnimationStateMachine::new(
///             starting_state,
///             my_states_map,
///             my_states_transitions_vec,
///             state_machine_vars,
///         ));
/// }
/// ```
#[derive(Component, Default, Reflect, FromReflect)]
#[reflect(Component)]
pub struct AnimationStateMachine {
    current_state: String,
    states: HashMap<String, AnimationState>,
    transitions: Vec<StateMachineTransition>,
    variables: StateMachineVariables,
}

impl AnimationStateMachine {
    /// Creates a new [`AnimationStateMachine`]
    pub fn new<T: ToString>(
        current_state: T,
        states: HashMap<T, AnimationState>,
        transitions: Vec<StateMachineTransition>,
        variables: HashMap<T, StateMachineVariableType>,
    ) -> Self {
        Self {
            current_state: current_state.to_string(),
            states: states
                .iter()
                .map(|(name, state)| (name.to_string(), state.to_owned()))
                .collect(),
            transitions,
            variables: variables
                .iter()
                .map(|(name, var)| (name.to_string(), var.to_owned()))
                .collect(),
        }
    }

    #[inline]
    fn current_state(&self) -> Option<AnimationState> {
        self.get_state(&self.current_state)
    }

    fn get_state(&self, state_name: &String) -> Option<AnimationState> {
        match self.states.contains_key(state_name) {
            true => Some(self.states[state_name].to_owned()),
            false => None,
        }
    }

    fn transitions_from_state(&self, state_name: &String) -> Vec<StateMachineTransition> {
        self.transitions
            .iter()
            .filter(|t| {
                t.start_state == AnimationStateRef::StateName(state_name.to_owned())
                    || t.start_state.is_any()
            })
            .map(|t| t.to_owned())
            .collect()
    }

    fn transitions_from_current_state(&self) -> Vec<StateMachineTransition> {
        self.transitions_from_state(&self.current_state)
    }

    fn animation_finished(
        player: &AnimationPlayer,
        state: &AnimationState,
        animations: &Assets<AnimationClip>,
    ) -> bool {
        match animations.get(&state.clip) {
            Some(clip) => player.elapsed() >= clip.duration(),
            None => true,
        }
    }

    /// Updates the value of the given variable
    pub fn update_variable<T: ToString>(&mut self, name: T, value: StateMachineVariableType) {
        self.variables.insert(name.to_string(), value);
    }
}

/// [`AnimationStateMachine`] state structure
#[derive(Default, Debug, Clone, Reflect, FromReflect)]
pub struct AnimationState {
    /// Animation clip handle
    pub clip: Handle<AnimationClip>,
    /// State name
    pub name: String,
    /// If set to `true`, the animation will only be interrupted once any valid transition is triggered
    pub interruptible: bool,
}

impl AnimationState {
    fn state_ref(&self) -> AnimationStateRef {
        AnimationStateRef::StateName(self.name.to_owned())
    }
}

/// Reference to an [`AnimationState`] name
#[derive(Debug, Clone, PartialEq, Eq, Reflect, FromReflect)]
pub enum AnimationStateRef {
    /// Wildcard reference
    AnyState,
    /// Reference to a specific state
    StateName(String),
}

impl AnimationStateRef {
    /// Creates a [`AnimationStateRef`] from a `impl ToString` value
    pub fn from_string<T: ToString>(name: T) -> Self {
        Self::StateName(name.to_string())
    }

    #[inline]
    fn unwrap(&self) -> &String {
        match self {
            Self::AnyState => panic!("Unexpected AnimationStateRef::AnyState"),
            Self::StateName(state) => state,
        }
    }

    /// Tests if self equals to [`AnimationStateRef::AnyState`]
    pub fn is_any(&self) -> bool {
        matches!(self, Self::AnyState)
    }
}

impl Display for AnimationStateRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AnyState => write!(f, "AnyState"),
            Self::StateName(state_name) => write!(f, "{state_name}"),
        }
    }
}

/// Transition from [`AnimationState`] A to [`AnimationState`] B
///
/// Example
/// ```
/// # use bevy_simple_state_machine::{StateMachineTransition, StateMachineTrigger, AnimationStateRef};
/// let transition = StateMachineTransition {
///     start_state: AnimationStateRef::from_string("idle"),
///     end_state: AnimationStateRef::from_string("run"),
///     trigger: StateMachineTrigger::from(|vars| vars["run"].is_bool(true)),
/// };
/// ```
#[derive(Clone, Reflect, FromReflect)]
pub struct StateMachineTransition {
    /// Reference to the starting state
    pub start_state: AnimationStateRef,
    /// Reference to the end state
    ///
    /// ## Note
    /// Do not set this to [`AnimationStateRef::AnyState`], or it may panic
    pub end_state: AnimationStateRef,
    /// Transition trigger condition
    #[reflect(ignore)]
    pub trigger: StateMachineTrigger,
}

impl Display for StateMachineTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "StateMachineTransition({} -> {})",
            self.start_state, self.end_state,
        )
    }
}

/// Trigger condition for a [`StateMachineTransition`]
///
/// Current values are:
///  - Never: the transition is never executed
///  - Always: the transition is always executed. This happens on the next frame or once the previous animation has concluded
///  - Condition: supports a custom condition of type `Fn(&StateMachineVariables) -> bool + Send + Sync`
///
/// Example
/// ```
/// # use bevy_simple_state_machine::StateMachineTrigger;
/// // this trigger returns true if the state machine variable "run" is set to true
/// let trigger = StateMachineTrigger::from(|vars| vars["run"].is_bool(true));
/// ```
#[derive(Default, Clone)]
pub enum StateMachineTrigger {
    /// The transition is never executed
    #[default]
    Never,
    /// The transition is always executed. This happens on the next frame or once the previous animation has concluded
    Always,
    /// The transition is executed once the given function evaluates to `true`
    Condition(Arc<dyn Fn(&StateMachineVariables) -> bool + Send + Sync>),
}

impl StateMachineTrigger {
    /// Creates a new [`StateMachineTrigger::Condition`] from the given function
    ///
    /// Example
    /// ```
    /// # use bevy_simple_state_machine::StateMachineTrigger;
    /// // this trigger returns true if the state machine variable "run" is set to true
    /// let trigger = StateMachineTrigger::from(|vars| vars["run"].is_bool(true));
    /// ```
    pub fn from(f: impl Fn(&StateMachineVariables) -> bool + Send + Sync + 'static) -> Self {
        Self::Condition(Arc::new(f))
    }

    /// Internal function to evaluate the state of a trigger
    fn evaluate(&self, variables: &StateMachineVariables) -> bool {
        match self {
            Self::Never => false,
            Self::Always => true,
            Self::Condition(f) => (f)(variables),
        }
    }
}

/// Event emitted once a [`StateMachineTransition`] has been executed
///
/// ## Note
/// Transitions right now conclude on the same frame they are triggered  
#[derive(Debug, Clone)]
pub struct TransitionEndedEvent {
    /// The entity on which the transition has been executed
    pub entity: Entity,
    /// Reference to the origin [`AnimationState`]
    pub origin: AnimationStateRef,
    /// Reference to the end [`AnimationState`]
    pub end: AnimationStateRef,
}
