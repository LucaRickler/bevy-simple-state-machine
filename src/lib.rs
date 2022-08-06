use std::fmt::{Debug, Display};

use bevy::{prelude::*, utils::HashMap};

pub struct SimpleStateMachinePlugin {}

impl Plugin for SimpleStateMachinePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<StateMachineEvent>()
            .add_system(Self::check_transitions);
    }
}

impl SimpleStateMachinePlugin {
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
            )
        }
    }
}

#[derive(Component, Default)]
pub struct AnimationStateMachine {
    current_state: String,
    states: HashMap<String, AnimationState>,
    transitions: Vec<StateMachineTransition>,
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
                    if transition.trigger.as_ref()() {
                        if let Some(next_state) = self.get_state(transition.end_state.unwrap()) {
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
}

#[derive(Default, Debug, Clone)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnimationStateRef {
    AnyState,
    StateName(String),
}

impl AnimationStateRef {
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

#[derive(Clone)]
pub struct StateMachineTransition {
    pub start_state: AnimationStateRef,
    pub end_state: AnimationStateRef,
    pub trigger: Box<dyn StateMachineTrigger>,
}

impl Debug for StateMachineTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "StateMachineTransition({} -> {}): {}",
            self.start_state,
            self.end_state,
            self.trigger.as_ref()()
        )
    }
}

pub trait StateMachineTrigger: Fn() -> bool + Send + Sync + ClonedStateMachineTrigger {}

pub trait ClonedStateMachineTrigger {
    fn clone_box(&self) -> Box<dyn StateMachineTrigger>;
}

impl<T> ClonedStateMachineTrigger for T
where
    T: 'static + StateMachineTrigger + Clone,
{
    fn clone_box(&self) -> Box<dyn StateMachineTrigger> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn StateMachineTrigger> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

pub struct StateMachineEvent {
    pub entity: Entity,
    pub origin: AnimationStateRef,
    pub end: AnimationStateRef,
}
