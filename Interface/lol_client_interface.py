from lcu_driver import Connector
from tensorflow import keras
import requests
import asyncio
import numpy as np


model = keras.models.load_model("AI/model")
connector = Connector()

lol_version = requests.get("https://ddragon.leagueoflegends.com/api/versions.json").json()[0]
champ_data = requests.get("https://ddragon.leagueoflegends.com/cdn/" + lol_version + "/data/en_US/champion.json").json()["data"]
CHAMPS = []
for champ in champ_data:
    CHAMPS.append((champ, int(champ_data[champ]["key"])))
CHAMPS.sort(key=lambda x: x[1])
NUM_CHAMPS = len(champ_data)


def champid_to_index(champ_id):
    for i in range(NUM_CHAMPS):
        if CHAMPS[i][1] == champ_id:
            return i


def champid_to_champ(champ_id):
    for champ in CHAMPS:
        if champ[1] == champ_id:
            return champ[0]


def champ_id_list_to_one_hot(champ_list):
    one_hot = [0]*NUM_CHAMPS
    for champ in champ_list:
        one_hot[champid_to_index(champ)] = 1
    return one_hot


# fired when LCU API is ready to be used
@connector.ready
async def connect(connection):
    print('LCU API is ready to be used.')


# fired when League Client is closed (or disconnected from websocket)
@connector.close
async def disconnect(_):
    print('The client have been closed!')
    await connector.stop()


def combinations(list1, n):
    out = []
    if n == 1:
        return [[elem] for elem in list1]
    for i in range(len(list1)):
        out += [[list1[i]] + combination for combination in combinations(list1[i+1:], n-1)]
    return out


def filter_champ_select_data(data):
    team = {}
    for player in data["myTeam"]:
        team[player["summonerId"]] = player["championId"]
    bench = data["benchChampionIds"]
    return team, bench


def filter_lobby_data(data):
    premades = []
    premades.append(data["localMember"]["summonerId"])
    for member in data["members"]:
        id = member["summonerId"]
        if id != premades[0]:
            premades.append(id)
    return premades


# subscribe to '/lol-summoner/v1/current-summoner' endpoint for the UPDATE event
# when an update to the user happen (e.g. name change, profile icon change, level, ...) the function will be called
#@connector.ws.register('/lol-summoner/v1/current-summoner', event_types=('UPDATE',))
@connector.ws.register('/lol-champ-select/v1/session', event_types=('UPDATE',))
async def champ_select_changed(connection, event):
    global premades
    team, bench = filter_champ_select_data(event.data)
    my_id = premades[0]
    my_champ = team[my_id]
    rest_champs = [team[id] for id in team if id != my_id]
    print()
    print("Your choices:")
    eval_teams(bench + [my_champ], rest_champs)
    if len(premades) > 1:
        print("premades:")
        premade_champs = [team[id] for id in premades]
        randoms_champs = [team[id] for id in team if id not in premades]
        eval_teams(bench + premade_champs, randoms_champs)


@connector.ws.register('/lol-lobby/v2/lobby', event_types=('UPDATE',))
async def champ_select_changed(connection, event):
    global premades
    premades = filter_lobby_data(event.data)


def eval_teams(bench, team):
    comps = [team + combination for combination in combinations(bench, 5-len(team))]
    alternatives = np.array([champ_id_list_to_one_hot(comp) for comp in comps])
    chances = model.predict(alternatives)
    comp_win_pairs = [(comps[i][len(team):], chances[i][0]) for i in range(len(comps))]
    comp_win_pairs.sort(key=lambda x: -x[1])
    print([champid_to_champ(champ) for champ in team])
    for comp in comp_win_pairs:
        champs = [champid_to_champ(champ) for champ in comp[0]]
        prob = comp[1]
        print("Win chance with {}: {:.1%}".format(champs, prob))


def update_team(new_team):
    team = new_team


def update_bench(new_bench):
    bench = new_bench

premades = []

connector.start()