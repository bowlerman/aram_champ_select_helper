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
CHAMPS = [(champ, int(champ_data[champ]["key"])) for champ in champ_data]
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
    return [index_to_champ(i) for i in range(len(one_hot)) if one_hot[i]]


def champ_id_list_to_one_hot(champ_list):
    one_hot = [0]*NUM_CHAMPS
    for champ in champ_list:
        one_hot[champid_to_index(champ)] = 1
    return one_hot


def champ_list_to_one_hot(champ_list):
    one_hot = [0]*NUM_CHAMPS
    for champ in champ_list:
        one_hot[champ_to_index(champ)] = 1
    return one_hot


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
    tf.keras.layers.Dense(256, activation='sigmoid', input_shape=(NUM_CHAMPS,)),
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
model.save("AI/model", save_format=tf.keras.experimental.export_saved_model)
"""
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



def champ_list_is_valid(champ_list):
    return all(champ_to_index(champ) is not None for champ in champ_list)


def combinations(list1, n):
    out = []
    if n == 1:
        return [[elem] for elem in list1]
    for i in range(len(list1)):
        out += [[list1[i]] + combination for combination in combinations(list1[i+1:], n-1)]
    return out

# Temp terminal app for aram champ select
team = []
bench = []
while True:
    command = input("> ")
    commands = command.split()
    if commands == []:
        continue
    elif commands[0] == "team":
        if champ_list_is_valid(commands[1:]):
            team = commands[1:]
            print("Team champs updated")
        else:
            print("Please input 4 champs (capitals matter)")
    elif commands[0] == "bench":
        if champ_list_is_valid(commands[1:]):
            bench = commands[1:]
            print("Bench updated")
        else:
            print("Please input champions (capitals matter)")
    elif commands[0] == "premades":
        premades = int(commands[1])
        print("Premade count updated")
    elif commands[0] == "eval":
        if len(team) == 5:
            comp = np.array([champ_list_to_one_hot(team), ])
            chance = model.predict(comp)[0][0]
            print("Win chance with {}:  {:.1%}".format(team, chance))
            continue
        comps = [team + combination for combination in combinations(bench, 5-len(team))]
        alternatives = np.array([champ_list_to_one_hot(comp) for comp in comps])
        chances = model.predict(alternatives)
        comp_win_pairs = [(comps[i][len(team):], chances[i][0]) for i in range(len(comps))]
        comp_win_pairs.sort(key=lambda x: -x[1])
        for comp in comp_win_pairs:
            print("Win chance with {}: {:.1%}".format(*comp))
"""
