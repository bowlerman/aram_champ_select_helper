import requests
import typing # noqa F401
import time

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
        raise requests.HTTPError('did not get match response, status code: ',
                                 r.status_code)


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
    for participant in match_info['participants']:
        if participant['teamId'] == 100:
            team_100_champs.append(participant['championId'])
        else:
            team_200_champs.append(participant['championId'])
    for team in match_info['teams']:
        if team['win'] == 'Win':
            winner = team['teamId']
            break
    data = {'win': winner, 100: team_100_champs, 200: team_200_champs}
    return data


number_of_aram_games = 0
valid_request_counter = 0
number_of_requests = 20
match_id = 2901255157
match_info = []
number_of_game_ids_not_found = 0
t0_20_request = time.time()
t0_100_request = time.time()
for _ in range(number_of_requests):
    try:
        match_info.append(get_match_info(str(match_id)))
    except requests.HTTPError:
        number_of_game_ids_not_found += 1
    if is_aram(match_info[-1]):
        number_of_aram_games += 1
    match_id += 1
    valid_request_counter += 1
    if valid_request_counter % 20 == 0:
        diff = time.time() - t0_20_request
        if diff < 1:
            time.sleep(1-diff)
        t0_20_request = time.time()
    if valid_request_counter % 100 == 0:
        diff = time.time() - t0_100_request
        if diff < 120:
            time.sleep(120-diff)
        t0_100_request = time.time()


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
    #testing number of aram games
    print('\nthe number of aram games: ', number_of_aram_games)
    #end test
