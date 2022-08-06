use std::{
    fmt::{Debug, Display},
    sync::Arc,
};

use bevy::{prelude::*, reflect::FromReflect, utils::HashMap};

pub struct SimpleStateMachinePlugin {}

impl Plugin for SimpleStateMachinePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<StateMachineEvent>()
            .register_type::<AnimationStateMachine>()
            .register_type::<AnimationStateRef>()
            .add_system(Self::check_transitions.label(StateMachineSystemLabel::StateMachineLabel))
            .add_system(
                Self::init_state_machines.label(StateMachineSystemLabel::StateMachineLabel),
            );
    }
}

impl SimpleStateMachinePlugin {
    pub fn new() -> Self {
        Self {}
    }

    fn check_transitions(
        mut state_machines_query: Query<(Entity, &mut AnimationStateMachine, &mut AnimationPlayer)>,
        animations: Res<Assets<AnimationClip>>,
        mut event_writer: EventWriter<StateMachineEvent>,
    ) {
        for (entity, mut state_machine, mut player) in &mut state_machines_query {
            state_machine.check_transitions(
                entity,
                &mut player,
                animations.as_ref(),
                &mut event_writer,
            );
        }
    }

    fn init_state_machines(
        mut state_machines_query: Query<
            (&AnimationStateMachine, &mut AnimationPlayer),
            Added<AnimationStateMachine>,
        >,
    ) {
        for (state_machine, mut player) in &mut state_machines_query {
            state_machine.init(&mut player);
        }
    }
}

#[derive(SystemLabel, Clone)]
pub enum StateMachineSystemLabel {
    StateMachineLabel,
}

pub type StateMachineVariables = HashMap<String, bool>;

#[derive(Component, Default, Reflect, FromReflect)]
#[reflect(Component)]
pub struct AnimationStateMachine {
    pub current_state: String,
    pub states: HashMap<String, AnimationState>,
    pub transitions: Vec<StateMachineTransition>,
    pub variables: StateMachineVariables,
}

impl AnimationStateMachine {
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
            .filter(|t| t.start_state == AnimationStateRef::StateName(state_name.to_owned()))
            .map(|t| t.to_owned())
            .collect()
    }

    fn transitions_from_current_state(&self) -> Vec<StateMachineTransition> {
        self.transitions_from_state(&self.current_state)
    }

    fn check_transitions(
        &mut self,
        entity: Entity,
        player: &mut AnimationPlayer,
        animations: &Assets<AnimationClip>,
        event_writer: &mut EventWriter<StateMachineEvent>,
    ) {
        if let Some(current_state) = self.current_state() {
            if current_state.interruptible
                || Self::animation_finished(player, &current_state, animations)
            {
                for transition in self.transitions_from_current_state() {
                    if transition.trigger.evaluate(&self.variables) {
                        if let Some(next_state) = self.get_state(transition.end_state.unwrap()) {
                            debug!("triggering {}", transition);
                            self.current_state = next_state.name;
                            player.play(next_state.clip);
                            event_writer.send(StateMachineEvent {
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

    pub fn update_variable<T: ToString>(&mut self, name: T, value: bool) {
        self.variables.insert(name.to_string(), value);
    }

    fn init(&self, player: &mut AnimationPlayer) {
        if let Some(current_state) = self.current_state() {
            player.play(current_state.clip);
        }
    }
}

#[derive(Default, Debug, Clone, Reflect, FromReflect)]
pub struct AnimationState {
    pub clip: Handle<AnimationClip>,
    pub name: String,
    pub interruptible: bool,
}

impl AnimationState {
    fn state_ref(&self) -> AnimationStateRef {
        AnimationStateRef::StateName(self.name.to_owned())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Reflect, FromReflect)]
pub enum AnimationStateRef {
    AnyState,
    StateName(String),
}

impl AnimationStateRef {
    pub fn from_string<T: ToString>(name: T) -> Self {
        Self::StateName(name.to_string())
    }

    #[inline]
    pub fn unwrap(&self) -> &String {
        match self {
            Self::AnyState => panic!("Unexpected AnimationStateRef::AnyState"),
            Self::StateName(state) => state,
        }
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

#[derive(Clone, Reflect, FromReflect)]
pub struct StateMachineTransition {
    pub start_state: AnimationStateRef,
    pub end_state: AnimationStateRef,
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

#[derive(Default, Clone)]
pub enum StateMachineTrigger {
    #[default]
    Never,
    Always,
    Condition(Arc<dyn Fn(&StateMachineVariables) -> bool + Send + Sync>),
}

impl StateMachineTrigger {
    pub fn from(f: impl Fn(&StateMachineVariables) -> bool + Send + Sync + 'static) -> Self {
        Self::Condition(Arc::new(f))
    }

    fn evaluate(&self, variables: &StateMachineVariables) -> bool {
        match self {
            Self::Never => false,
            Self::Always => true,
            Self::Condition(f) => (f)(variables),
        }
    }
}

#[derive(Debug)]
pub struct StateMachineEvent {
    pub entity: Entity,
    pub origin: AnimationStateRef,
    pub end: AnimationStateRef,
}