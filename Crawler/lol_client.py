import requests
r = requests.get("https://127.0.0.1:2999/liveclientdata/allgamedata", verify=False)

r.json()
print(r.json())