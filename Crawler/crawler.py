import cassiopeia as cass
import arrow
from pymongo import MongoClient
client = MongoClient('localhost', 27017)
DB = client.aram_champ_select_helper

with open('Crawler/api_key.txt', 'r') as api_key_file:
    api_key = api_key_file.readline()
cass.set_riot_api_key(api_key)
cass.set_default_region("EUW")


def store_match_data(processed_match_info):
    match_data = DB.match_data
    match_data.insert_one(processed_match_info)


def is_aram(match_info):
    if match_info['queueId'] == 65 or match_info['queueId'] == 450:
        return True
    return False


def process_data(match_info):
    """extracts data necessary for AI

    Args:
        match_info (dict): match data

    Returns:
        dict: processed match data
    """
    team_100_champs = []
    team_200_champs = []
    winner = ''
    patch = match_info["version"]
    # print(match_info)
    match_id = match_info['id']
    for participant in match_info["participants"]:
        if participant['side'].value == 100:
            team_100_champs.append(participant['championId'])
        else:
            team_200_champs.append(participant['championId'])
    for team in match_info['teams']:
        if team['isWinner']:
            winner = team['side'].value
            break
    data = {'match_id': match_id, 'win': winner, '100': team_100_champs,
            '200': team_200_champs, 'patch': patch}
    return data

def store_summoner(account_id):
    summoners = DB.summoners
    summoners.update_one({"account_id": account_id}, { "$setOnInsert": {"account_id": account_id, "time_at_last_fetch": 0}}, upsert = True)


def fetch_match_history(account_id, begin_time):
    summoner = cass.Summoner(account_id=account_id, region="EUW")
    match_history = cass.MatchHistory(summoner=summoner, queues={
                                      cass.Queue.aram}, begin_time=begin_time)
    return match_history


def match_in_db(match):
    match_data = DB.match_data
    return not match_data.find_one({"match_id": match.id}) is None


def insert_match_history(account_id, begin_time):
    cached_summoner = DB.summoners.find_one({"account_id": account_id})
    if not (cached_summoner is None):
        begin_time = max(arrow.get(cached_summoner["time_at_last_fetch"]), begin_time)
    match_history = fetch_match_history(account_id, begin_time)
    if len(match_history) == 0 and begin_time < AGE_LIMIT:
        DB.summoners.delete_one({"account_id": account_id})
        return
    for match in match_history:
        if not match_in_db(match):
            store_match_data(process_data(match.load().to_dict()))
            for player in match.participants:
                store_summoner(player.to_dict()["accountId"])
    DB.summoners.replace_one({"account_id": account_id}, {"account_id": account_id, "time_at_last_fetch": arrow.now().int_timestamp})


def get_oldest_summoner():
    return DB.summoners.find().sort("time_at_last_fetch", direction=1).limit(1).next()["account_id"]


AGE_LIMIT = arrow.get(arrow.now().int_timestamp-300000)
DEFAULT_START_TIME = cass.Patch.from_date(AGE_LIMIT).start

insert_match_history("IDEt30cqSuvTUdVJNYoQ3goSJsgtxlHsl7OlcqrLh9DyjA", DEFAULT_START_TIME)
while True:
    try:
        account_id = get_oldest_summoner()
        insert_match_history(account_id, DEFAULT_START_TIME)
    except Exception as e:
        print(e)
