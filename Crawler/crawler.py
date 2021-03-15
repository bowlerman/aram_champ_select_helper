import requests
import time
import logging
from pymongo import MongoClient
client = MongoClient('localhost', 27017)
db = client.aram_champ_select_helper
# Create a custom logger
logger = logging.getLogger(__name__)


# Create handlers
c_handler = logging.StreamHandler()
f_handler = logging.FileHandler('Crawler/crawler.log')
c_handler.setLevel(logging.WARNING)
f_handler.setLevel(logging.ERROR)
# Create formatters and add it to handlers


c_format = logging.Formatter('%(name)s - %(levelname)s - %(message)s')
f_format = logging.Formatter('%(asctime)s - %(name)s - %(levelname)s - %(message)s')
c_handler.setFormatter(c_format)
f_handler.setFormatter(f_format)


# Add handlers to the logger
logger.addHandler(c_handler)
logger.addHandler(f_handler)
with open('Crawler/api_key.txt', 'r') as api_key_file:
    api_key = api_key_file.readline()


def get_match_info(match_id: str):
    platform_url = 'https://euw1.api.riotgames.com'
    match_api_url = '/lol/match/v4/matches/'
    match_id = match_id
    r = requests.get(platform_url + match_api_url + match_id,
                     params={'api_key': api_key})
    if r.status_code == 200:
        return r.json()
    else:
        e = requests.HTTPError('status code: ', r.status_code)
        e.response = r
        raise e


def store_match_data(processed_match_info):
    match_data = db.match_data
    match_data.insert_one(processed_match_info)


def is_aram(match_info):
    if match_info['queueId'] == 65:
        return True
    return False


def data_processing(match_info):
    """extracts data necessary for AI

    Args:
        match_info (dict): match data

    Returns:
        dict: processed match data
    """
    team_100_champs = []
    team_200_champs = []
    winner = ''
    match_id = match_info['gameId']
    for participant in match_info['participants']:
        if participant['teamId'] == 100:
            team_100_champs.append(participant['championId'])
        else:
            team_200_champs.append(participant['championId'])
    for team in match_info['teams']:
        if team['win'] == 'Win':
            winner = team['teamId']
            break
    data = {'match_id': match_id, 'win': winner, '100': team_100_champs,
            '200': team_200_champs}
    return data


request_counter = 0
number_of_requests = 10000
match_id = 3201256855
t0_20_request = time.time()
t0_100_request = time.time()

while True:
    if request_counter % 500 == 0:
        print(time.strftime('%Y-%m-%d %H:%M:%S', time.localtime(1347517370)),
              'match id: ', match_id, 'number of requests so far: ',
              request_counter, '\n')
    request_counter += 1
    if request_counter % 20 == 0:
        diff = time.time() - t0_20_request
        if diff < 1:
            time.sleep(1-diff)
        t0_20_request = time.time()
    if request_counter % 100 == 0:
        diff = time.time() - t0_100_request
        if diff < 120:
            time.sleep(120-diff)
        t0_100_request = time.time()
    try:
        match_info = get_match_info(str(match_id))
        if is_aram(match_info):
            match_info = data_processing(match_info)
            store_match_data(match_info)
    except requests.HTTPError as e:
        if e.response.status_code == 504:
            time.sleep(1)
            continue
        if e.response.status_code == 429:
            time.sleep(1)
            continue
        elif e.response.status_code != 404:
            logger.error(match_id, exc_info=True)
    match_id += 1


if __name__ == '__main__':
    # start test: testing match id
    test_match_id = '2901155157'
    match_info = get_match_info(test_match_id)
    if 'gameCreation' in match_info:
        if match_info['gameCreation'] == 1477325559029:
            print('get_match_info function, passed')
        else:
            print('\nwrong id, get_match_info function failed, correct id: ',
                  match_info['gameCreation'])
    # end test: testing match id
    # start test: processing data
    test_match_id = '2901155157'
    match_info = get_match_info(test_match_id)
    data = data_processing(match_info)
    if data == {'win': 100, 100: [15, 1, 267, 19, 13],
                200: [78, 89, 45, 67, 64]}:
        print('\ndata processing test passed')
    else:
        print('\ndata processing test failed')
    # end test:
