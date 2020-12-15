import requests
import typing

with open('Crawler/api_key.txt', 'r') as api_key_file:
    api_key = api_key_file.readline()

def get_match_info(match_id:str):
    platform_url = 'https://euw1.api.riotgames.com'
    match_api_url = '/lol/match/v4/matches/'
    match_id = match_id
    r = requests.get(platform_url+ match_api_url+ match_id, params={'api_key':api_key})
    return r.json()
requests_count = 3
match_id = 2901155157
match_info = []
for _ in range(requests_count):
    match_id += 1 
    match_info.append(get_match_info(str(match_id)))
print('length of list: ', len(match_info))
if __name__ == '__main__':
    ## start test: testing match id
    test_match_id = '2901155157'
    match_info = get_match_info(test_match_id)
    if 'gameCreation' in match_info:
        if match_info['gameCreation'] == 1477325559029:
            print('get_match_info function, passed')
        else:
            print('wrong id, get_match_info function failed, correct id: ', match_info['gameCreation'])
    ## end test: testing match id