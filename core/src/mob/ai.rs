//! Mob "AI" (eye-I) stuff.

use serde::{Deserialize, Serialize};

use crate::{identity::MachineId, room::environ::{SpecialEnvironment, Terrain, WEATHER_RAIN}, traits::TickMeaning};

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq)]
pub enum AiState {
    Idle,
    Wandering,
}

impl Default for AiState {
    fn default() -> Self {
        Self::Idle
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq)]
pub enum AiMentalState {
    Happy,
    Neutral,
    Grumpy,
    Angry,
}

impl Default for AiMentalState {
    fn default() -> Self {
        Self::Neutral
    }
}

#[derive(Debug, Clone)]
pub enum AiAction {
    Emote { ent_m_id: MachineId, fmt: &'static str },
    //Attack { xyz }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Ai {
    state: AiState,
    //tick_id: usize,
    mental_state: AiMentalState,

    #[serde(skip, default = "ai_rng_default")] rng: u64,
    p_idle_to_wandering: f32,
    p_wandering_to_idle: f32,
}

pub(crate) fn ai_rng_default() -> u64 {
    rand::random::<u64>()
}

/// `seed` the "RNG"; return the new value.
#[inline] fn ai_rng(seed: u64) -> u64 { seed.wrapping_mul(6364136223846793005).wrapping_add(1) }
/// Generate a [0.0, 1.0] range value from give `base`.
// Perfect 1.0 is rare, but… can happen, thus the range isn't [0.0, 1.0)
#[inline] const fn ai_probability(base: u64) -> f32 { (base >> 32) as f32 / 4294967296.0 }
/// Check vs [0.0, 1.0] range whether to do something…
#[inline] const fn ai_do(base: u64, chance: f32) -> bool { ai_probability(base) <= chance }

impl Default for Ai {
    fn default() -> Self {
        Self { 
            state: AiState::default(),
            //tick_id: rand::random::<u64>() as usize,
            mental_state: AiMentalState::default(),
            rng: ai_rng_default(),
            p_idle_to_wandering: 0.01,
            p_wandering_to_idle: 0.05,
        }
    }
}

impl Ai {
    /// Tick the AI.
    /// 
    // By default we (usually) run AI at some fraction of the parent's [Room]'s Hz.
    pub fn tick(&mut self, e_tick_id: MachineId, curr_tick: usize, room_env: SpecialEnvironment, room_terrain: Option<Terrain>) -> Option<TickMeaning> {
        let mut maybe_state = None;
        let mut maybe_mental_state = None;
        let mut maybe_action = None;
        
        // Idle → Wander → Idle switcharoo.
        self.rng = ai_rng(self.rng);
        match self.state {
            AiState::Idle => {
                if ai_do(self.rng, 0.01) {
                    self.state = AiState::Wandering;
                    maybe_state = self.state.into();
                }
            }

            AiState::Wandering => {
                // move?
                if ai_do(self.rng, 0.05) {
                    self.state = AiState::Idle;
                    maybe_state = self.state.into();
                }
            }
        }

        self.rng = ai_rng(self.rng);
        // Some weather related checks…
        if room_env & WEATHER_RAIN != 0 {
            match self.mental_state {
                AiMentalState::Grumpy =>
                    if ai_do(self.rng, 0.005) {
                        maybe_action = AiAction::Emote { ent_m_id: e_tick_id, fmt: "[~e~] glares at the clouds for a moment." }.into()
                    }
                ,
                AiMentalState::Neutral =>
                    if ai_do(self.rng, 0.2) {
                        self.mental_state = AiMentalState::Grumpy;
                        maybe_mental_state = self.mental_state.into();
                    }
                ,
                AiMentalState::Happy => {
                    if ai_do(self.rng, 0.05) {
                        self.mental_state = AiMentalState::Neutral;
                        maybe_mental_state = self.mental_state.into();
                    }
                }
                // a bit of rain doesn't make Angry any angrier…
                _ => ()
            }
        } else {
            match self.mental_state {
                AiMentalState::Grumpy => {
                    if ai_do(self.rng, 1.0/3.0) {
                        self.mental_state = AiMentalState::Neutral;
                        maybe_mental_state = self.mental_state.into();
                    }
                }

                AiMentalState::Neutral => {
                    if ai_do(self.rng, 0.02) {
                        self.mental_state = AiMentalState::Happy;
                        maybe_mental_state = self.mental_state.into();
                    }
                }
                // no change for Angry
                _ => ()
            }
        }
        
        if  maybe_action.is_none() &&
            maybe_mental_state.is_none() &&
            maybe_state.is_none() { None }
        else {
            TickMeaning::AiStateChange { maybe_state, maybe_mental_state, maybe_action }.into()
        }
    }
}
