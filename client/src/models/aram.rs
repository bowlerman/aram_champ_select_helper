use anyhow::Error;

use std::{collections::HashMap, fs::File};
use tract_onnx::prelude::*;

use crate::{ARAMChampSelectState, Champ};

type OnnxModel = SimplePlan<TypedFact, Box<dyn TypedOp>, Graph<TypedFact, Box<dyn TypedOp>>>;

#[derive(Debug, Clone)]
pub struct ARAMAIModel {
    model: OnnxModel,
    champ_dict: HashMap<Champ, usize>,
}

impl ARAMAIModel {
    pub fn get_win_rate(&self, team: &[Champ; 5]) -> Result<f32, Error> {
        let tot_champs = self.champ_dict.len();
        let mut one_hot = vec![0_f32; tot_champs + 1];
        for champ in team {
            one_hot[self.champ_dict.get(champ).cloned().unwrap_or(tot_champs)] = 1.;
        }
        let input = tract_ndarray::arr1(&one_hot).into_shape((1, tot_champs + 1))?;
        let tensor_res = self
            .model
            .run(tvec![input.into()])?
            .to_vec()
            .pop()
            .expect("Expecting model output");
        let res: [f32; 2] = tensor_res.as_slice()?.try_into()?;
        let sum = res[0] + res[1];
        Ok(res[0] / sum * 100.)
    }

    pub fn new() -> Result<ARAMAIModel, Error> {
        let champs: Vec<(String, u16)> =
            serde_json::from_reader(File::open("model-trainer/champs.json").unwrap()).unwrap();
        let champs: Vec<(String, Champ)> = champs.into_iter().map(|(name, id)| (name, Champ::from(id))).collect();
        let tot_champs = champs.len();
        let champ_dict = map_champ_id_to_index(&champs).unwrap();
        let model = tract_onnx::onnx()
            // load the model
            .model_for_path("./model-trainer/model.onnx")?
            // specify input type and shape
            .with_input_fact(
                0,
                InferenceFact::dt_shape(f32::datum_type(), tvec![1, tot_champs + 1]),
            )?
            // optimize graph
            .into_optimized()?
            // make the model runnable and fix its inputs and outputs
            .into_runnable()?;
        Ok(ARAMAIModel { model, champ_dict })
    }
}

fn map_champ_id_to_index(all_champs: &[(String, Champ)]) -> Result<HashMap<Champ, usize>, Error> {
    let mut map = HashMap::new();
    for (i, &(_, champ_id)) in all_champs.iter().enumerate() {
        map.insert(champ_id, i);
    }
    Ok(map)
}

struct ARAMDilemma {
    model: ARAMAIModel,
    state: ARAMChampSelectState,
}
/*
impl Dilemma for ARAMDilemma {
    type Choice = Champ;
    fn choices(&self) -> Vec<u16> {
        self.state.choices()
    }

    fn eval(&self, &choice: &Self::Choice) -> Result<f32, Error> {
        self.model.get_win_rate(&[choice, self.state.bench[0], self.state.bench[1], self.state.bench[2], self.state.bench[3]])
    }

    fn repr_choice(&'_ self, &choice: &Self::Choice) -> LazyNodes {
        rsx!(choice)
    }
}*/
