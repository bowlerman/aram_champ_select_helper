import tensorflow as tf
import requests
from pymongo import MongoClient
import numpy as np

client = MongoClient('localhost', 27017)
db = client.aram_champ_select_helper
match_data = db.match_data


lol_version = requests.get("https://ddragon.leagueoflegends.com/api/versions.json").json()[0]
champ_data = requests.get("https://ddragon.leagueoflegends.com/cdn/" + lol_version + "/data/en_US/champion.json").json()["data"]
CHAMPS = []
for champ in champ_data:
    CHAMPS.append((champ, int(champ_data[champ]["key"])))
CHAMPS.sort(key=lambda x: x[1])

NUM_CHAMPS = len(champ_data)


def champ_to_index(champ):
    for i in range(NUM_CHAMPS):
        if CHAMPS[i][0] == champ:
            return i


def index_to_champ(index):
    return CHAMPS[index][0]


def champid_to_index(champ_id):
    for i in range(NUM_CHAMPS):
        if CHAMPS[i][1] == champ_id:
            return i


def champid_to_champ(champ_id):
    for champ in CHAMPS:
        if champ[1] == champ_id:
            return champ[0]


x = []
y = []

for document in match_data.find({}, {'100': 1, '200': 1, 'win': 1, '_id': 0}):
    for team in ['100', '200']:
        team_comp = [0]*NUM_CHAMPS
        for champ in document[team]:
            team_comp[champid_to_index(champ)] = 1
        x.append(team_comp)
        if str(document['win']) == team:
            y.append([1, 0])
        else:
            y.append([0, 1])

x = np.array(x)
y = np.array(y)

l = 2*len(x) // 3
x_train = x[:l]
y_train = y[:l]
x_test = x[l:]
y_test = y[l:]

x_new = [[0]*NUM_CHAMPS, [0]*NUM_CHAMPS]
for i in range(5):
    x_new[0][7*i+3] = 1
for i in range(4):
    x_new[1][7*i+3] = 1
x_new[1][114] = 1
print(x_new)
champname = []

for l in range(len(x_new)):
    for index in range(len(x_new[l])):
        print(index)
        if x_new[l][index] == 1:
            champname.append(index_to_champ(index))
x_new = np.array(x_new)
model = tf.keras.models.Sequential([
    tf.keras.Input(shape=(NUM_CHAMPS,)),
    tf.keras.layers.Dense(128, activation='relu'),
    tf.keras.layers.Dropout(0.2),
    tf.keras.layers.Dense(64, activation='relu'),
    tf.keras.layers.Dense(2, activation='softmax', name='result')
])

model.compile(optimizer='adam',
              loss='binary_crossentropy',
              metrics=['accuracy'])

model.fit(x_train, y_train, epochs=20)
model.evaluate(x_test, y_test)
print(model(x[:4]), y[:2])
print(model(x_new))
for champ_id in match_data.find({}, {'100': 1, '200': 1, 'win': 1, '_id': 0})[0]['100']:
    print(champid_to_champ(champ_id))


print(champname)
