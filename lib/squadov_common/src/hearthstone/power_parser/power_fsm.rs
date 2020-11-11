mod default_state;
mod create_game;
mod full_entity;
mod block_state;
mod tag_change;
mod null_state;
mod show_entity;

use crate::hearthstone::power_parser::PowerLog;
use crate::hearthstone::game_state::{HearthstoneGameLog, HearthstoneGameAction};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use regex::Regex;
use std::rc::Rc;
use std::cell::RefCell;
use uuid::Uuid;
use derive_more::{Display};

/// An action is represented in the logs as an all-caps string that
/// starts the line (after any whitespace).
#[derive(Display,Debug)]
pub enum PowerFsmAction {
    CreateGame,
    CreateGameEntity,
    CreatePlayerEntity,
    FullEntity,
    TagChange,
    BlockStart,
    BlockEnd,
    ShowEntity,
    HideEntity,
    ShuffleDeck,
    Metadata,
    SubSpellStart,
    SubSpellEnd,
    CachedTagForDormantChange,
    Unknown
}

impl std::str::FromStr for PowerFsmAction {
    type Err = crate::SquadOvError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "CREATE_GAME" => PowerFsmAction::CreateGame,
            "FULL_ENTITY" => PowerFsmAction::FullEntity,
            "TAG_CHANGE" => PowerFsmAction::TagChange,
            "BLOCK_START" => PowerFsmAction::BlockStart,
            "BLOCK_END" => PowerFsmAction::BlockEnd,
            "SHOW_ENTITY" => PowerFsmAction::ShowEntity,
            "HIDE_ENTITY" => PowerFsmAction::HideEntity,
            "SHUFFLE_DECK" => PowerFsmAction::ShuffleDeck,
            "META_DATA" => PowerFsmAction::Metadata,
            "SUB_SPELL_START" => PowerFsmAction::SubSpellStart,
            "SUB_SPELL_END" => PowerFsmAction::SubSpellEnd,
            "CACHED_TAG_FOR_DORMANT_CHANGE" => PowerFsmAction::CachedTagForDormantChange,
            _ => return Err(crate::SquadOvError::NotFound)
        })
    }
}

pub fn is_fsm_action_block_end(a: &PowerFsmAction) -> bool {
    match a {
        PowerFsmAction::BlockEnd => true,
        _ => false
    }
}

pub struct PowerFsmStateInfo {
    pub uuid: Uuid,
    pub tm: DateTime<Utc>,
    pub tags: HashMap<String, String>,
    pub attrs: HashMap<String, String>
}

impl PowerFsmStateInfo {
    pub fn new(tm: &DateTime<Utc>) -> Self {
        Self {
            uuid: Uuid::new_v4(),
            tm: tm.clone(),
            tags: HashMap::new(),
            attrs: HashMap::new(),
        }
    }
}

// We have a "hierarchical" finite state machine where we expect to start the parsing at some root state
// and ultimately end up in this root state as well. When we go from a parent state to a child state,
// we call 'on_leave_state_to_child' on the parent and 'on_enter_state_from_parent'. When the child state
// finishes and wishes to go back to the parent, we call 'on_leave_state_to_parent' on the child and
// 'on_enter_state_from_child' on the parent.
trait PowerFsmState {
    fn get_state_uuid(&self) -> Uuid;
    fn get_state_action(&self) -> PowerFsmAction;

    fn generate_hearthstone_game_actions(&self) -> Option<Vec<HearthstoneGameAction>> { None }

    fn on_enter_state_from_parent(&mut self, _previous: &mut Box<dyn PowerFsmState>) {}
    fn on_enter_state_from_child(&mut self, _previous: &mut Box<dyn PowerFsmState>) {}
    fn on_leave_state_to_child(&mut self) {}
    fn on_leave_state_to_parent(&mut self) {}

    fn handle_tag_attribute(&mut self, _tag: &str, _val: &str) {}
    // can_receive_action is called whenever we parse a line that contains an
    // action denoted in PowerFsmAction (which should contain the whole list of possible
    // actions we can encounter). If this returns false it indicates that this should
    // be the start of a new action.
    fn can_receive_action(&self, _action: &PowerFsmAction) -> bool { false }
}

#[derive(Display,Debug)]
#[display(fmt="PowerLogAction[Action:{} Attrs:{:?}]", action, attrs)]
struct PowerLogAction {
    action: PowerFsmAction,
    attrs: HashMap<String, String>
}

fn parse_action_attributes(attrs: &str) -> HashMap<String, String> {
    // Will have the form of KEY1=VALUE1 KEY2=VALUE2...
    // Note that it's *NOT* safe to split on whitespace as values may have
    // spaces in them. What we can do instead is to find each instance of an equal sign
    // and then move *back* from the equal sign to find the space to be able to fully parse out the key from the previous
    // value. Once we've down this, then we can split each token off the equal sign to pull out
    // each individual key-value pair. Note that doing this isn't 100% safe either as there may be 
    // equal signs within the VALUE too. UGH. I'm going to go ahead and make the assumption that
    // if there *is* an equal sign within a value then it'll be surrounded by brackets [].
    let mut tokens: Vec<String> = Vec::new();

    let mut ptr : Option<usize> = Some(attrs.len());
    while ptr.is_some() {
        let prev_ptr = ptr.unwrap();
        let rest = &attrs[0..prev_ptr];
        let lbracket = rest.rfind('[');
        let rbracket = rest.rfind(']');
        ptr = rest.rfind('=');
     
        if ptr.is_some() {
            let pidx = ptr.unwrap();
            // If the equal sign is within the brackets then we want to
            // find the first equal sign to the left of the brackets. 
            if lbracket.is_some() && rbracket.is_some() {
                let lidx = lbracket.unwrap();
                let ridx = rbracket.unwrap();
                if pidx > lidx && pidx < ridx {
                    ptr = attrs[0..lidx].rfind('=');
                }
            }

            match attrs[0..ptr.unwrap()].rfind(' ') {
                Some(ws_idx) => {
                    tokens.push(attrs[ws_idx+1..prev_ptr].to_string());
                    ptr = Some(ws_idx);
                },
                None => {
                    tokens.push(attrs[0..prev_ptr].to_string());
                    ptr = None;
                }
            };
        }
    }

    let mut ret : HashMap<String, String> = HashMap::new();
    for tk in &tokens {
        let split: Vec<&str> = tk.split('=').collect();
        ret.insert(split[0].trim().to_string(), split[1..].join("=").trim().to_string());
    }

    return ret;
}

impl PowerLogAction {
    fn from_log(log: &PowerLog) -> Option<Self> {
        // Most actions are all capital letters with underscores with the exception of the
        // CREATE_GAME GameEntity and Player actions so we handle those first before handling the more
        // generic one.
        if log.log.starts_with("GameEntity") {
            Some(Self {
                action: PowerFsmAction::CreateGameEntity,
                attrs: parse_action_attributes(log.log.strip_prefix("GameEntity").unwrap_or(""))
            })
        } else if log.log.starts_with("Player") {
            Some(Self {
                action: PowerFsmAction::CreatePlayerEntity,
                attrs: parse_action_attributes(log.log.strip_prefix("Player").unwrap_or(""))
            })
        } else {
            let action_tokens : Vec<&str> = log.log.split_whitespace().collect();
            let action = match action_tokens[0].parse() {
                Ok(x) => x,
                Err(_) => return None
            };

            Some(Self {
                action: action,
                attrs: parse_action_attributes(&action_tokens[1..].join(" "))
            })
        }
    }
}

struct PowerLogTagAttribute {
    tag: String,
    value: String
}

impl PowerLogTagAttribute {
    fn from_log(log: &PowerLog) -> Option<Self> {
        lazy_static! {
            static ref RE: Regex = Regex::new("tag=(.*) value=(.*)").unwrap();
        }
        let captures = match RE.captures(&log.log) {
            Some(x) => x,
            None => return None
        };
        
        Some(Self {
            tag: captures.get(1).map_or("", |m| m.as_str()).to_string(),
            value: captures.get(2).map_or("", |m| m.as_str()).to_string(),
        })
    }
}

fn power_log_action_to_fsm_state(tm: &DateTime<Utc>, action: &PowerLogAction, st: Rc<RefCell<HearthstoneGameLog>>) -> Box<dyn PowerFsmState> {
    let mut info = PowerFsmStateInfo::new(tm);
    info.attrs = action.attrs.clone();

    match action.action {
        PowerFsmAction::CreateGame => Box::new(create_game::CreateGameState::new(info)),
        PowerFsmAction::CreateGameEntity => Box::new(create_game::CreateGameEntityState::new(info)),
        PowerFsmAction::CreatePlayerEntity => Box::new(create_game::CreateGamePlayerState::new(info)),
        PowerFsmAction::FullEntity => Box::new(full_entity::FullEntityState::new(info)),
        PowerFsmAction::ShowEntity => Box::new(show_entity::ShowEntityState::new(info)),
        PowerFsmAction::TagChange => Box::new(tag_change::TagChangeState::new(info)),
        PowerFsmAction::BlockStart => Box::new(block_state::BlockState::new(info, st)),
        _ => Box::new(null_state::NullState::new())
    }
}

struct RawLog {
    tm: DateTime<Utc>,
    log: String,
    state_name: String,
    state_uuid: Uuid
}

/// The PowerFsm struct is a stateful way of keeping track of the actions performed
/// by the players in a match as recorded by the power log. The power log is composed
/// of multiple "blocks" where each "block" can contain a number of "actions" and/or "blocks".
/// Note that it is possible for an "action" to live outside of a "block". Since we read in
/// the log one line at a time and any particular "action" and "block" can span multiple lines
/// we'll use a FSM to keep track of what action/block we're in and that action/block is 
/// responsible for parsing in future lines and determining what to do next. Every time control
/// is returned back to the default FSM state, we assume a logical unit of actions has token place.
/// Note that each FSM state is responsible for modifying the current game state as necessary.
pub struct PowerFsm {
    store_raw: bool,
    raw: Vec<RawLog>,
    states: Vec<Box<dyn PowerFsmState>>,
    pub game: Rc<RefCell<HearthstoneGameLog>>
}

impl PowerFsm {
    pub fn new(store_raw: bool) -> Self {
        let game = Rc::new(RefCell::new(HearthstoneGameLog::new()));
        Self {
            store_raw: store_raw,
            raw: Vec::new(),
            states: vec![Box::new(default_state::DefaultPowerState::new(game.clone()))],
            game: game.clone(),
        }
    }

    pub fn raw_logs_to_string(&self) -> String {
        let mut logs : Vec<String> = Vec::new();
        for raw in &self.raw {
            logs.push(format!("{}\t{}\t{}\t{}", &raw.tm, &raw.log, &raw.state_name, &raw.state_uuid))
        }
        return logs.join("\n");
    }

    pub fn finish(&mut self) {
        self.game.borrow_mut().finish();
    }

    pub fn parse(&mut self, tm : &DateTime<Utc>, log: &PowerLog) -> Result<(), crate::SquadOvError> {
        let mut parsed = false;
        
        // Check whether the input log line is an action or a tag attribute.
        let parsed_action = PowerLogAction::from_log(log);
        if parsed_action.is_some() {
            // There should always at least be the default power state so this unwrap is safe.
            let current_state = self.states.last_mut().unwrap();
            let parsed_action = parsed_action.unwrap();

            // A new action means we go to a new state. The only question at this point
            // is whether or not we need to leave the current state.
            let can_receive = current_state.can_receive_action(&parsed_action.action);
            if !can_receive && self.states.len() > 1 {
                self.pop_current_state();
            }

            // At this point we should be able to push a state and make it the new current state.
            // Note that the exception to this rule are the actions that denote an end of the block in which case
            // popping the latest block off is sufficient.
            if !is_fsm_action_block_end(&parsed_action.action) {
                let next_state = power_log_action_to_fsm_state(tm, &parsed_action, self.game.clone());
                self.push_state(next_state);
            } else if self.states.len() > 1 {
                // If it is an block end then we need to pop off ANOTHER state as the previous pop off
                // only popped off the latest action within the block. However, we also need to handle
                // the possibility of empty blocks so only do another pop if the current state
                self.pop_current_state();
            }

            parsed = true;
        }

        if !parsed {
            let parsed_tag_attr = PowerLogTagAttribute::from_log(log);
            if parsed_tag_attr.is_some() {
                let parsed_tag_attr = parsed_tag_attr.unwrap();
                // There should always at least be the default power state so this unwrap is safe.
                let current_state = self.states.last_mut().unwrap();
                current_state.handle_tag_attribute(&parsed_tag_attr.tag, &parsed_tag_attr.value);
                parsed = true;
            }
        }

        if !parsed {
            // At this point we've encountered an unknown line. Oh well!
            log::warn!("Unknown Power Log Line: {}", log.log);
        }

        if self.store_raw {
            let current_state = self.states.last_mut().unwrap();
            self.raw.push(RawLog{
                tm: tm.clone(),
                log: log.log.clone(),
                state_name: current_state.get_state_action().to_string(),
                state_uuid: current_state.get_state_uuid()
            });
        }
        
        Ok(())
    }

    fn pop_current_state(&mut self) {
        let mut current_state = self.states.pop().unwrap();
        let parent_state = self.states.last_mut().unwrap();

        current_state.on_leave_state_to_parent();
        parent_state.on_enter_state_from_child(&mut current_state);
    }

    fn push_state(&mut self, mut st: Box<dyn PowerFsmState>) {
        let current_state = self.states.last_mut().unwrap();
        current_state.on_leave_state_to_child();
        st.on_enter_state_from_parent(current_state);
        self.states.push(st);
    }
}