use std::{fmt::Debug, time::Duration, path::Path, future::Future};

use anyhow::Error;
use iced::{pure::{Element, Application, widget::{Text, Column, Button, Image, Row}, Widget, image}, Subscription, Command, executor, Settings, Renderer, Length};
use iced_futures::{backend::default::time, MaybeSend};
use logic::{lol_client_api::ChampSelectFetcher, ARAMChampSelectState};
use logic::Champ;

use once_cell::sync::OnceCell;
mod logic;
mod gui;

trait Dilemma<A: Application>
where A::Message: 'static {
    type Choice;

    fn choices(&self) -> Vec<Self::Choice>;

    fn eval(&self, choice: &Self::Choice) -> Result<f32, Error>;

    fn repr_choice(&'_ self, choice: Self::Choice) -> Element<'_, A::Message>;

    fn repr_choice_with_win(&'_ self, choice: Self::Choice) -> Element<'_, A::Message> {
        if let Ok(win_rate) = self.eval(&choice) {
            Column::new()
                .push(self.repr_choice(choice))
                .push(Text::new(format!("{:.2}%", win_rate * 100.0)))
                .into()
        }
        else {
            Column::new()
                .push(self.repr_choice(choice))
                .push(Text::new("Error"))
                .into()
        }
    }

    fn repr_choices_with_win(&'_ self) -> Element<'_, A::Message>
    {
        let mut col = Column::new();
        for choice in self.choices() {
            col = col.push(self.repr_choice_with_win(choice));
        }
        col.into()
    }
}

fn ok_ref<T, E>(res: &Result<T, E>) -> Option<&T> {
    match res {
        Ok(val) => Some(val),
        Err(_) => None,
    }
}
/*
#[allow(non_snake_case)]
fn ChampSelect<Fetcher: ChampSelectFetcher + Clone + 'static>(
    cx: Scope<ChampSelectProps<Fetcher>>,
) -> Element {
    let fetcher = cx.props.fetcher.clone();
    let champ_select_state_handle: &UseState<Option<ARAMChampSelectState>> = use_state(&cx, || None);
    let champ_select_state_store = champ_select_state_handle.clone();
    use_coroutine::<(), _, _>(&cx, |_| async move {
        let mut wait = interval(Duration::from_millis(1000));
        loop {
            let (maybe_champ_select_state, _) =
                tokio::join!(fetcher.get_champ_select_state(), wait.tick());
            if ok_ref(&maybe_champ_select_state)
                != champ_select_state_store.current().as_ref().as_ref()
            {
                champ_select_state_store.set(maybe_champ_select_state.ok())
            }
        }
    });
    let model = use_state(&cx, || ARAMAIModel::new().unwrap()); //application is useless without valid model
    let model = model.current();
    let state = champ_select_state_handle.current().as_ref().clone();
    match state {
        None => cx.render(rsx!("Waiting for champ select")),
        Some(champ_select_state) => {
            let team_champs = champ_select_state.team_champs;
            let win_rates = champ_select_state
                .choices()
                .into_iter()
                .filter_map(|choice| {
                    Some((
                        choice,
                        model
                            .get_win_rate(&[
                                choice,
                                team_champs[0],
                                team_champs[1],
                                team_champs[2],
                                team_champs[3],
                            ])
                            .ok()?,
                    ))
                });
            let win_rate_displays = win_rates.map(|(choice, win_rate)| {
                rsx!(ARAMWinRateDisplay {
                    champ: choice,
                    win_rate: win_rate
                })
            });
            cx.render(rsx!(div {
                display: "flex",
                flex_wrap: "wrap",
                win_rate_displays
            }))
        }
    }
}
*/

