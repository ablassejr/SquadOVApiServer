import argparse
import json
import requests

def testValorant(ip, session):
    url = 'http://{0}:8080/v1/valorant'.format(ip)
    headers = {
        'x-squadov-session-id': session,
        'Content-Type': 'application/json',
    }
    packet = {
        'matchId': '00000000-0000-0000-0000-000000000000',
    }

    resp = requests.post(url, json=packet, headers=headers).json()
    print('VALORANT RESPONSE: ', resp)

def testHearthstone(ip, userId, session):
    print("TEST HEARTHSTONE ", ip, userId, session)
    url = 'http://{0}:8080/v1/hearthstone/user/{1}/match'.format(ip, userId)
    headers = {
        'x-squadov-session-id': session,
        'Content-Type': 'application/json',
    }

    with open('data/hearthstone_data.json') as f:
        data = json.load(f)

    resp = requests.post(url, json=data, headers=headers).json()
    print('HEARTHSTONE RESPONSE: ', resp)

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--mode', required=True)
    parser.add_argument('--session', required=True)
    parser.add_argument('--user', type=int, default=0)
    parser.add_argument('--ip', required=True)
    args = parser.parse_args()

    if args.mode == 'valorant':
        testValorant(args.ip, args.session)
    elif args.mode == 'hearthstone':
        testHearthstone(args.ip, args.user, args.session)