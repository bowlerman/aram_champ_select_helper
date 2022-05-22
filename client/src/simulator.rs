use std::io::stdin;

use super::{Champ, ChampSelectState, ChampSelectFetcher};
use clap::{Parser, Subcommand, AppSettings};
use lazy_static::lazy_static;
use std::sync::Mutex;
use async_trait::async_trait;
use anyhow::Error;

#[derive(Parser, Debug)]
#[clap(global_setting(AppSettings::NoBinaryName))]
struct Cli{
    #[clap(subcommand)]
    command: Commands
}

lazy_static! {
    pub static ref CHAMP_SELECT_STATE: Mutex<ChampSelectState> = Mutex::new(Default::default());
}

#[derive(Debug, Clone)]
pub struct FakeChampSelectFetcher {}

#[async_trait]
impl ChampSelectFetcher for FakeChampSelectFetcher {
    async fn get_champ_select_state(&self) -> Result<ChampSelectState, Error> {
        Ok(CHAMP_SELECT_STATE.lock().unwrap().to_owned())
    }

    async fn new() -> Self {
        FakeChampSelectFetcher {}
    }
}

#[derive(Subcommand, Debug)]
enum Commands{
    AddBench {champ: Champ},
    RmBench,
    YourChamp {champ: Champ},
    TeamChamps {pos: usize, champ: Champ},
    Print,
}

use Commands::*;

pub async fn simulator() {
    loop {
        let mut buffer = String::new();
        stdin().read_line(&mut buffer).unwrap();
        let maybe_cli = Cli::try_parse_from(buffer.split_whitespace());
        let cli = match maybe_cli {
            Ok(cli) => cli,
            Err(err) => {err.print().unwrap(); continue}
        };
        let mut champ_select_state = CHAMP_SELECT_STATE.lock().unwrap();
        match cli.command {
            AddBench{champ} => {
                champ_select_state.bench.push(champ)
            },
            RmBench => {
                champ_select_state.bench.pop();
            },
            YourChamp{champ} => {
                champ_select_state.your_champ = champ
            },
            TeamChamps{pos, champ} => {
                champ_select_state.team_champs[pos] = champ
            },
            Print => ()
        }
        println!("{champ_select_state:?}");
    }
}