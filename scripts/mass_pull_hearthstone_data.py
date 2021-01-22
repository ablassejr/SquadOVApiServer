import argparse
import os
import json
import requests
import base64
import time

class BlizzardApiClient:
    def __init__(self, clientId, clientSecret):
        self.clientId = clientId
        self.clientSecret = clientSecret
        self.refreshAccessToken()        

    def refreshAccessToken(self):
        url = "https://us.battle.net/oauth/token"
        headers = {
            'Authorization': 'Basic {0}'.format(
                base64.b64encode((self.clientId + ':' + self.clientSecret).encode('utf-8')).decode('utf-8')
            )
        }
        data = {
            'grant_type': 'client_credentials'
        }
        
        resp = requests.post(url, data=data, headers=headers).json()
        self.accessToken = resp["access_token"]

    def generateAccessTokenHeaders(self):
        return {
            'Authorization': 'Bearer {0}'.format(self.accessToken)
        }

    # TODO: Handle other locales?
    def fetchHearthstoneCard(self, id):
        url = 'https://us.api.blizzard.com/hearthstone/cards/{0}?locale=en_US'.format(id)
        resp = requests.get(url, headers=self.generateAccessTokenHeaders())
        if resp.status_code != 200:
            return (None, resp.status_code == 404)
        return (resp.json(), False)

    def fetchHearthstoneCardBack(self, id):
        url = 'https://us.api.blizzard.com/hearthstone/cardbacks/{0}?locale=en_US'.format(id)
        resp = requests.get(url, headers=self.generateAccessTokenHeaders())
        if resp.status_code != 200:
            return (None, resp.status_code == 404)
        return (resp.json(), False)

def handleCard(client, cardFolder):
    print('Card: ', cardFolder)
    metadataFname = os.path.join(cardFolder, 'metadata.json')
    with open(metadataFname, encoding='utf-8') as f:
        metadata = json.load(f)

    imageFname = os.path.join(cardFolder, 'card.png')
    if os.path.exists(imageFname):
        return True
    
    cardId = metadata["Dbf"]["Id"]
    print('\tID: ', cardId)
    (data, notExists) = client.fetchHearthstoneCard(cardId)
    
    if notExists:
        return True

    if data is None:
        return False

    # data has two fields we care about: image and imageGold
    # which have pre-renderered cards we can use.

    if "image" in data and data["image"] != '':
        image = requests.get(data["image"]).content
        with open(imageFname, 'wb') as f:
            f.write(image)

    imageGoldFname = os.path.join(cardFolder, 'cardGold.png')
    if "imageGold" in data and data["imageGold"] != '':
        imageGold = requests.get(data["imageGold"]).content
        with open(imageGoldFname, 'wb') as f:
            f.write(imageGold)
    
    return True

def handleCardBack(client, backFolder):
    print('Card Back: ', backFolder)
    metadataFname = os.path.join(backFolder, 'metadata.json')
    with open(metadataFname, encoding='utf-8') as f:
        metadata = json.load(f)

    cardBackId = metadata["Id"]
    print('\tID: ', cardBackId)
    (data, notExists) = client.fetchHearthstoneCardBack(cardBackId)
    
    if notExists:
        return True

    if data is None:
        return False

    imageFname = os.path.join(backFolder, 'back.png')
    if os.path.exists(imageFname):
        return True
    if "image" in data and data["image"] != '':
        image = requests.get(data["image"]).content
        with open(imageFname, 'wb') as f:
            f.write(image)

    return True

def handleCardFolder(client, folder):
    for cd in os.listdir(folder):
        while not handleCard(client, os.path.join(folder, cd)):
            time.sleep(0.015)

def handleCardBackFolder(client, folder):
    for cd in os.listdir(folder):
        while not handleCardBack(client, os.path.join(folder, cd)):
            time.sleep(0.015)

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--folder', required=True)
    parser.add_argument('--clientId', required=True)
    parser.add_argument('--clientSecret', required=True)
    args = parser.parse_args()

    clientId = args.clientId
    clientSecret = args.clientSecret
    client = BlizzardApiClient(clientId, clientSecret)

    # We expect a folder that has the structure
    # FOLDER
    #   -- Cards
    #       -- CARD_ID
    #           -- portrait.png
    #           -- metadata.json
    #   -- CardBacks
    #       -- CARD_BACK_ID
    #           -- texture.png
    #           -- metadata.json
    # For every card and card back we query Blizzard's API to grab a pre-rendered card/card back
    # so we don't have to worry about ourselves.
    cardFolder = os.path.join(args.folder, 'Cards')
    handleCardFolder(client, cardFolder)

    cardBackFolder = os.path.join(args.folder, 'CardBacks')
    handleCardBackFolder(client, cardBackFolder)