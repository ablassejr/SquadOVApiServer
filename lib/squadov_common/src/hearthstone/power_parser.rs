pub mod power_fsm;
use crate::hearthstone::{HearthstoneRawLog, GameType, FormatType};
use chrono::{DateTime, Utc};
use std::fmt;
use std::collections::HashMap;

pub struct HearthstoneGameState {
    pub game_type: GameType,
    pub format_type: FormatType,
    pub scenario_id: i32,
    player_map: HashMap<i32, String>
}

impl Default for HearthstoneGameState {
    fn default() -> Self {
        Self {
            game_type: GameType::Unknown,
            format_type: FormatType::Unknown,
            scenario_id: 0,
            player_map: HashMap::new()
        }
    }
}

impl fmt::Display for HearthstoneGameState {
    fn fmt(&self,  f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[Game: {} Format: {} Scenario: {} Players: {:?}]", self.game_type, self.format_type, self.scenario_id, self.player_map)
    }
}

pub struct PowerLog {
    pub func: String,
    pub log: String,
    pub indent_level: i32
}

impl PowerLog {
    fn new(input: &HearthstoneRawLog) -> Option<Self> {
        let tokens: Vec<&str> = input.log.splitn(2, '-').collect();

        if tokens.len() == 2 {
            let mut indent_level = 0;
            for c in tokens[1].chars() {
                if c.is_whitespace() {
                    indent_level += 1
                } else {
                    break
                }
            }

            Some(Self {
                func: tokens[0].trim().to_string(),
                log: tokens[1].trim().to_string(),
                indent_level
            })
        } else {
            None
        }
    }
}

pub struct HearthstonePowerLogParser {
    // State that doesn't change past the first time we find it in the logs.
    pub state: HearthstoneGameState,

    // FSM for parsing game state power lines. This FSM records all the tasks
    // performed as well as snapshots of the state of the game at certain points.
    pub fsm: power_fsm::PowerFsm,
}

impl HearthstonePowerLogParser {
    pub fn new(store_raw: bool) -> Self {
        Self {
            state: HearthstoneGameState{
                ..Default::default()
            },
            fsm: power_fsm::PowerFsm::new(store_raw)
        }
    }

    pub fn parse(&mut self, logs: &[HearthstoneRawLog]) -> Result<(), crate::SquadOvError> {
        for log in logs {
            // Parse the log even further into the power log format.
            let pl = PowerLog::new(log);

            if pl.is_none() {
                continue;
            }
            let pl = pl.unwrap();

            if self.parse_game_state_print_game(&pl)? {
                continue;
            }

            if self.parse_game_state_print_power(&log.time, &pl)? {
                continue;
            }
        }
        self.fsm.finish();
        return Ok(())
    }

    fn parse_game_state_print_game(&mut self, log: &PowerLog) -> Result<bool, crate::SquadOvError> {
        if log.func != "GameState.DebugPrintGame()" {
            return Ok(false);
        }

        let tokens: Vec<&str> = log.log.split('=').collect();
        if tokens[0] == "GameType" {
            self.state.game_type = tokens[1].parse()?;
        } else if tokens[0] == "FormatType" {
            self.state.format_type = tokens[1].parse()?;
        } else if tokens[0] == "ScenarioID" {
            self.state.scenario_id = tokens[1].parse()?;
        } else if tokens[0] == "PlayerID" {
            // Special case where we're splitting a log line that looks like
            // PlayerID=ID, PlayerName=PLAYER_NAME
            let resplit: Vec<&str> = log.log.split(", ").collect();
            let tokens: Vec<&str> = resplit[0].split('=').collect();
            let id = tokens[1].parse()?;

            let tokens: Vec<&str> = resplit[1].split('=').collect();
            let name: String = tokens[1].to_string();

            self.state.player_map.insert(id, name);

            // We need to update the FSM parser with this player map because some entity names are specified
            // using the player's name so we need to be able to map the player name to the entity ID.
            self.fsm.game.borrow_mut().set_player_map(&self.state.player_map);
        }

        return Ok(true);
    }

    fn parse_game_state_print_power(&mut self, tm : &DateTime<Utc>, log: &PowerLog) -> Result<bool, crate::SquadOvError> {
        if log.func != "GameState.DebugPrintPower()" {
            return Ok(false);
        }

        self.fsm.parse(tm, log)?;
        return Ok(true);
    }
}