pub mod lol_client_api;
pub mod models;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Champ {
    pub id: u16,
}

impl From<u16> for Champ {
    fn from(id: u16) -> Self {
        Self { id }
    }
}

impl TryFrom<String> for Champ {
    type Error = anyhow::Error;
    fn try_from(id: String) -> Result<Self, Self::Error> {
        Ok(Self { id: id.parse()? })
    }
}

impl From<Champ> for String {
    fn from(champ: Champ) -> Self {
        champ.id.to_string()
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ARAMChampSelectState {
    pub your_champ: usize,
    pub bench: Vec<Champ>,
    pub team_champs: [Champ; 5],
}

impl ARAMChampSelectState {
    pub fn choices(&self) -> Vec<Champ> {
        let mut ret = vec![self.team_champs[self.your_champ]];
        ret.extend(&self.bench);
        ret
    }

    pub fn add_bench(&mut self, champ: Champ) {
        self.bench.push(champ);
    }

    pub fn remove_bench(&mut self, index: usize) {
        if index < self.bench.len() {
            self.bench.remove(index);
        }
    }

    pub fn swap_bench(&mut self, index: usize, champ: Champ) {
        if index < self.bench.len() {
            self.bench[index] = champ;
        }
    }

    pub fn swap_team(&mut self, index: usize, champ: Champ) {
        if index < self.team_champs.len() {
            self.team_champs[index] = champ;
        }
    }
}

#[test]
fn test_choices() {
    let cs = ARAMChampSelectState {
        your_champ: 4,
        bench: vec![1, 51, 124, 12, 53].iter().map(|&x| x.into()).collect(),
        team_champs: [123, 12, 3, 1, 12].map(|x| x.into()),
    };
    assert_eq!(cs.choices().len(), cs.bench.len() + 1)
}
