import tensorflow as tf
import requests
from pymongo import MongoClient
import numpy as np
import random

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


def one_hot_to_champ_list(one_hot):
    champ_list = []
    for i in range(len(one_hot)):
        if one_hot[i]:
            champ_list.append(index_to_champ(i))
    return champ_list


x = []
y = []

for document in match_data.find({"patch": "11.5.361.3108"}, {'100': 1, '200': 1, 'win': 1, '_id': 0}):
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

model = tf.keras.models.Sequential([
    tf.keras.Input(shape=(NUM_CHAMPS,)),
    tf.keras.layers.Dense(256, activation='sigmoid'),
    tf.keras.layers.Dropout(0.2),
    tf.keras.layers.Dense(64, activation='sigmoid'),
    tf.keras.layers.Dense(64, activation='sigmoid'),
    tf.keras.layers.Dense(2, activation='softmax', name='result')
])

model.compile(optimizer='adam',
              loss='binary_crossentropy',
              metrics=['accuracy'])

model.fit(x_train, y_train, epochs=10)
model.evaluate(x_test, y_test)
predictions = model.predict(x_test)


# Checking if the certainty of the model is accurate
bound_interval = 0.05
bounds = [0.5 + i*bound_interval for i in range(round(0.5/bound_interval))]
for bound in bounds:
    count = 0
    correct = 0
    for i in range(len(predictions)):
        for j in [0, 1]:
            if bound < predictions[i][j] <= bound + bound_interval:
                count += 1
                if y_test[i][j]:
                    correct += 1
    if count:
        print("Model certainty:  {:.3}-{:.3}".format(bound, bound+bound_interval))
        print("Sample size:  {}".format(count))
        print("Correct guesses:  {}".format(correct))
        print("Percentage correct:  {:.1%}".format(correct/count))
        print()
