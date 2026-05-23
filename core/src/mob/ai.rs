//! Mob "AI" (eye-I) stuff.

use serde::{Deserialize, Serialize};

use crate::{identity::MachineId, mob::faction::{Demeanor, EntityFaction}, rng::*, room::environ::{SpecialEnvironment, Terrain, WEATHER_RAIN, WEATHER_STORM}, traits::TickMeaning};

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
    Giddy,
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
    Attack,// { xyz }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Ai {
    state: AiState,
    pub(super) mental_state: AiMentalState,

    #[serde(skip, default = "cg_rng_default")] rng: u64,
    p_idle_to_wandering: f32,
    p_wandering_to_idle: f32,
}

impl Default for Ai {
    fn default() -> Self {
        Self { 
            state: AiState::default(),
            //tick_id: rand::random::<u64>() as usize,
            mental_state: AiMentalState::default(),
            rng: cg_rng_default(),
            p_idle_to_wandering: 0.01,
            p_wandering_to_idle: 0.05,
        }
    }
}

impl Ai {
    /// Tick the AI.
    /// 
    // By default we (usually) run AI at some fraction of the parent's [Room]'s Hz.
    pub fn tick(&mut self,
            e_tick_id: MachineId,
            _curr_tick: usize,
            room_env: SpecialEnvironment,
            _room_ter: Option<Terrain>,
            faction: EntityFaction,
    ) -> Option<TickMeaning> {
        let mut maybe_state = None;
        let mut maybe_mental_state = None;
        let mut maybe_action = None;
        
        // Idle → Wander → Idle switcharoo.
        self.rng = cg_rng(self.rng);
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

        self.rng = cg_rng(self.rng);
        // Some weather related checks…
        if room_env & (WEATHER_RAIN|WEATHER_STORM) != 0 {
            // storm makes things angsty easier…
            let storm_mul = if room_env & WEATHER_STORM != 0 { 1.0 + 1.0/3.0 } else { 1.0 };
            
            match &self.mental_state {
                AiMentalState::Grumpy => {
                    if ai_do(self.rng, 0.005) {
                        maybe_action = AiAction::Emote { ent_m_id: e_tick_id, fmt: "[~e~] glares at the clouds for a moment." }.into();
                    }

                    self.rng = cg_rng(self.rng);
                    if ai_do(self.rng, 0.15) {
                        self.mental_state = AiMentalState::Angry;
                        maybe_mental_state = self.mental_state.into();
                    }
                }
                
                AiMentalState::Neutral =>
                    if ai_do(self.rng, 0.2 * storm_mul) {
                        self.mental_state = AiMentalState::Grumpy;
                        maybe_mental_state = self.mental_state.into();
                    }
                ,
                AiMentalState::Happy => {
                    if ai_do(self.rng, 0.05 * storm_mul) {
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

        if  matches!(faction, EntityFaction::NPC { demeanor: Demeanor::Hostile }) &&
            matches!(self.mental_state, AiMentalState::Angry|AiMentalState::Grumpy) &&
            maybe_action.is_none() {
            maybe_action = AiAction::Attack.into()
        }
        
        if  maybe_action.is_none() &&
            maybe_mental_state.is_none() &&
            maybe_state.is_none() { None }
        else {
            TickMeaning::AiStateChange { maybe_state, maybe_mental_state, maybe_action }.into()
        }
    }
}
