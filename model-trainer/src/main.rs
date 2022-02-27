use data_collection::{get_current_unix_time, MatchDocument};
use futures::stream::StreamExt;
use futures::Stream;
use futures_core::FusedFuture;
use mongodb::{bson::doc, options::ClientOptions, Client, Collection};
use riven::consts::Champion;
use std::{collections::HashMap, error::Error};
use tch::{nn, nn::Module, nn::OptimizerConfig, Device, Reduction, Tensor, kind::Element};
use neuroflow::{data::{DataSet, Extractable}, FeedForward};

const HIDDEN_NODES: i64 = 128;
const TOT_SIZE: i64 = 10000;
const TIME_LIMIT: i64 = 60 * 60 * 24 * 7;

fn net(vs: &nn::Path, tot_champs: i64) -> impl Module {
    nn::seq()
        .add(nn::linear(
            vs / "layer1",
            tot_champs + 1,
            HIDDEN_NODES,
            Default::default(),
        ))
        .add_fn(|xs| xs.relu())
        .add(nn::linear(vs, HIDDEN_NODES, 2, Default::default()))

}

async fn get_matches(
    match_collection: &Collection<MatchDocument>,
    time_bound: i64,
) -> Result<Vec<MatchDocument>, Box<dyn Error>> {
    let current_time = get_current_unix_time();
    let earliest_time = current_time - time_bound;
    let filter = doc! {"game_start" : {"$gt" : earliest_time} };
    let matches: Vec<MatchDocument> = match_collection
        .find(filter, None)
        .await?
        .filter_map(|matc| async move { matc.ok() })
        .collect()
        .await;
    Ok(matches)
}

fn champ_list_to_nodes(
    champ_list: [Champion; 5],
    all_champs: &HashMap<Champion, usize>,
) -> Vec<f64> {
    let tot_champs = all_champs.len();
    let mut ret = vec![0.; tot_champs + 1];
    for c in champ_list {
        if c.is_known() {
            ret[all_champs[&c]] = 1.;
        } else {
            ret[tot_champs] += 1.;
        }
    }
    ret
}

fn matches_to_nn_data(
    matches: Vec<MatchDocument>,
    all_champs: &HashMap<Champion, usize>,
) -> DataSet {
    let mut data = DataSet::new();
    for matc in matches {
        data.push(&champ_list_to_nodes(matc.blue_champs, all_champs), &[Into::<f64>::into(Into::<u8>::into(matc.blue_win)), Into::<f64>::into(Into::<u8>::into(!matc.blue_win))]);
        data.push(&champ_list_to_nodes(matc.red_champs, all_champs), &[Into::<f64>::into(Into::<u8>::into(!matc.blue_win)), Into::<f64>::into(Into::<u8>::into(matc.blue_win))]);
    }
    data
}

fn init_all_champs() -> HashMap<Champion, usize> {
    let mut all_champs: HashMap<Champion, usize> = HashMap::new();
    for i in 0..=1000 {
        if Champion::from(i).is_known() {
            all_champs.insert(Champion::from(i), all_champs.len());
        }
    }
    all_champs
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let all_champs = &init_all_champs();
    let tot_champs = all_champs.len();
    let client_options = ClientOptions::parse("mongodb://localhost:27017");
    let client = Client::with_options(client_options.await?)?;
    let db = &client.database("aram_champ_select_helper");
    let match_collection: &Collection<MatchDocument> = &db.collection("matches");
    let matches = get_matches(match_collection, TIME_LIMIT).await?;
    assert!(matches.len() != 0);
    let data = matches_to_nn_data(matches, all_champs);
    let mut net = FeedForward::new(&[(tot_champs + 1).try_into()?, 128, 2]);
    net.activation(neuroflow::activators::Type::Tanh).learning_rate(0.01).train(&data, 50_000);
    dbg!(net.calc(data.rand().0));
    Ok(())
}
