import tensorflow as tf
import requests
from pymongo import MongoClient
import numpy as np
import random
import time
import json
import shutil
from tensorflow.python.saved_model import builder as saved_model_builder
from tensorflow.python.saved_model.signature_def_utils import predict_signature_def
from tensorflow.python.saved_model import tag_constants
import onnxmltools

client = MongoClient('localhost', 27017)
db = client.aram_champ_select_helper
match_data = db.matches


lol_version = requests.get("https://ddragon.leagueoflegends.com/api/versions.json").json()[0]
champ_data = requests.get("https://ddragon.leagueoflegends.com/cdn/" + lol_version + "/data/en_US/champion.json").json()["data"]
CHAMPS = [(champ, int(champ_data[champ]["key"])) for champ in champ_data]
CHAMPS.sort(key=lambda x: x[1])
NUM_CHAMPS = len(CHAMPS)
MAX_TIME = 60*60*24*7

with open("model-trainer/champs.json", "w") as champ_file:
    json.dump(CHAMPS, champ_file)

def champ_to_index(champ) -> int:
    for i in range(NUM_CHAMPS):
        if CHAMPS[i][0] == champ:
            return i
    return NUM_CHAMPS


def index_to_champ(index):
    return CHAMPS[index][0]


def champid_to_index(champ_id) -> int:
    for i in range(NUM_CHAMPS):
        if CHAMPS[i][1] == champ_id:
            return i
    return NUM_CHAMPS


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

for document in match_data.find({"game_start": {"$gt": time.time()-MAX_TIME}}):
    for team in ['blue', 'red']:
        team_comp = [0]*(NUM_CHAMPS+1)
        for champ in document[team+"_champs"]:
            team_comp[champid_to_index(champ)] = 1
        x.append(team_comp)
        if document['blue_win']:
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
    tf.keras.layers.Dense(256, activation='sigmoid', input_shape=(NUM_CHAMPS+1,)),
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
onnx_model = onnxmltools.convert_keras(model)
onnxmltools.utils.save_model(onnx_model, 'model-trainer/model.onnx')

# Checking if the certainty of the model is accurate
bound_interval = 0.01
bounds = [0 + i*bound_interval for i in range(round(1/bound_interval))]
for bound in bounds:
    count = 0
    correct = 0
    for i in range(len(predictions)):
        for j in [0]:
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
