use data_collection::{get_current_unix_time, MatchDocument};
use futures::stream::StreamExt;
use futures::Stream;
use futures_core::FusedFuture;
use mongodb::{bson::doc, options::ClientOptions, Client, Collection};
use riven::consts::Champion;
use std::{collections::HashMap, error::Error};
use tch::{nn, nn::Module, nn::OptimizerConfig, Device, Reduction, Tensor, kind::Element};

const TOT_CHAMPS: i64 = 784;
const HIDDEN_NODES: i64 = 128;
const TOT_SIZE: i64 = 10000;
const TIME_LIMIT: i64 = 60 * 60 * 24 * 7;

fn net(vs: &nn::Path) -> impl Module {
    nn::seq()
        .add(nn::linear(
            vs / "layer1",
            TOT_CHAMPS + 1,
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
) -> Vec<f32> {
    let mut ret = [0.; (TOT_CHAMPS + 1) as usize];
    for c in champ_list {
        if c.is_known() {
            ret[all_champs[&c]] = 1.;
        } else {
            ret[TOT_CHAMPS as usize] += 1.;
        }
    }
    ret.to_vec()
}

fn matches_to_nn_data(
    matches: Vec<MatchDocument>,
    all_champs: &HashMap<Champion, usize>,
) -> (Tensor, Tensor,  Tensor, Tensor) {
    let ((train_data, train_labels), (test_data, test_labels)): ((Vec<Vec<f32>>, Vec<Vec<f32>>), (Vec<Vec<f32>>, Vec<Vec<f32>>)) = matches.into_iter().take(50000).map(|matc| {
        match rand::random() {
            true => (
                (champ_list_to_nodes(matc.blue_champs, all_champs),
                if matc.blue_win {vec![1., 0.]} else {vec![0., 1.]}),
                (champ_list_to_nodes(matc.red_champs, all_champs),
                if matc.blue_win {vec![0.]} else {vec![1.]}),
            ),
            false => (
                (champ_list_to_nodes(matc.red_champs, all_champs),
                if !matc.blue_win {vec![1., 0.]} else {vec![0., 1.]}),
                (champ_list_to_nodes(matc.blue_champs, all_champs),
                if !matc.blue_win {vec![0.]} else {vec![1.]}),
            )
        }
    }).unzip();
    assert!{train_data.len() != 0}
    assert!{test_data.len() != 0}
    (Tensor::of_slice2(&train_data), Tensor::of_slice2(&train_labels), Tensor::of_slice2(&test_data), Tensor::of_slice2(&test_labels))
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
    let client_options = ClientOptions::parse("mongodb://localhost:27017");
    let client = Client::with_options(client_options.await?)?;
    let db = &client.database("aram_champ_select_helper");
    let match_collection: &Collection<MatchDocument> = &db.collection("matches");
    let matches = get_matches(match_collection, TIME_LIMIT).await?;
    assert!(matches.len() != 0);
    let (train_data, train_labels, test_data, test_labels) = matches_to_nn_data(matches, all_champs);
    let vs = nn::VarStore::new(Device::cuda_if_available());
    let net = net(&vs.root());
    let mut opt = nn::Adam::default().build(&vs, 1e-3)?;
    for epoch in 1..10 {
        let loss = net.forward(&train_data).binary_cross_entropy_with_logits::<Tensor>(&train_labels, None, None, Reduction::Mean);
        opt.backward_step(&loss);
        dbg!{&test_data};
        dbg!{&test_labels};
        let test_accuracy = net.forward(&test_data);
        dbg!(2);
        let temp = test_accuracy.accuracy_for_logits(&test_labels);
        dbg!(3);
        let test_accuracy = temp;
        println!(
            "epoch: {:4} train loss: {:8.5} test acc: {:5.2}%",
            epoch,
            f64::from(&loss),
            100. * f64::from(&test_accuracy),
        );
    }
    Ok(())
}
