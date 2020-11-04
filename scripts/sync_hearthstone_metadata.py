import argparse
from sqlalchemy import create_engine, Table, Column, Integer, String, MetaData, Boolean, ForeignKey
from sqlalchemy.sql import text
from sqlalchemy.dialects.postgresql import insert
import os
import json

metadata = MetaData()

hearthstoneCards = Table('hearthstone_cards', metadata,
    Column('card_id', String, primary_key=True),
    Column('id', Integer, nullable=False),
    Column('has_card', Boolean, nullable=False),
    Column('has_golden', Boolean, nullable=False),
    schema='squadov',
)

hearthstoneCardNames = Table('hearthstone_card_names', metadata,
    Column('card_id', None, ForeignKey('squadov.hearthstone_cards.card_id')),
    Column('locale', String, nullable=False),
    Column('string', String, nullable=False),
    schema='squadov',
)

hearthstoneCardText = Table('hearthstone_card_text', metadata,
    Column('card_id', None, ForeignKey('squadov.hearthstone_cards.card_id')),
    Column('locale', String, nullable=False),
    Column('string', String, nullable=False),
    schema='squadov',
)

hearthstoneCardTags = Table('hearthstone_card_tags', metadata,
    Column('card_id', None, ForeignKey('squadov.hearthstone_cards.card_id')),
    Column('tag', Integer, nullable=False),
    Column('val', Integer, nullable=False),
    schema='squadov',
)

hearthstoneCardBacks = Table('hearthstone_card_backs', metadata,
    Column('id', Integer, primary_key=True),
    Column('has_back', Boolean, nullable=False),
    Column('active', Boolean, nullable=False),
    schema='squadov',
)

hearthstoneCardBackNames = Table('hearthstone_card_back_names', metadata,
    Column('back_id', None, ForeignKey('squadov.hearthstone_card_backs.id')),
    Column('locale', String, nullable=False),
    Column('string', String, nullable=False),
    schema='squadov',
)

hearthstoneCardBackDescriptions = Table('hearthstone_card_back_descriptions', metadata,
    Column('back_id', None, ForeignKey('squadov.hearthstone_card_backs.id')),
    Column('locale', String, nullable=False),
    Column('string', String, nullable=False),
    schema='squadov',
)

def handleCardFolder(cardFolder):
    metadataFname = os.path.join(cardFolder, 'metadata.json')
    with open(metadataFname, encoding='utf-8') as f:
        metadata = json.load(f)

    cardData = {
        'card_id': metadata["Dbf"]["CardId"],
        'id': metadata["Dbf"]["Id"],
        'has_card': os.path.exists(os.path.join(cardFolder, 'card.png')),
        'has_golden': os.path.exists(os.path.join(cardFolder, 'cardGold.png'))
    }

    cardNames = []
    for key, value in metadata["Dbf"]["Name"]["Value"].items():
        cardNames.append({
            'card_id': metadata["Dbf"]["CardId"],
            'locale': key,
            'string': value
        })

    cardTexts = []
    for key, value in metadata["Dbf"]["TextInHand"]["Value"].items():
        cardTexts.append({
            'card_id': metadata["Dbf"]["CardId"],
            'locale': key,
            'string': value
        })

    cardTags = []
    for key, value in metadata["Tags"]["SerializedTags"].items():
        cardTags.append({
            'card_id': metadata["Dbf"]["CardId"],
            'tag': int(key),
            'val': value
        })

    return cardData, cardNames, cardTexts, cardTags

def handleCardBackFolder(backFolder):
    metadataFname = os.path.join(backFolder, 'metadata.json')
    with open(metadataFname, encoding='utf-8') as f:
        metadata = json.load(f)

    backData = {
        'id': metadata["Id"],
        'has_back': os.path.exists(os.path.join(backFolder, 'back.png')),
        'active': metadata["Enabled"],
    }

    backNames = []
    for key, value in metadata["Name"]["Value"].items():
        backNames.append({
            'back_id': metadata["Id"],
            'locale': key,
            'string': value
        })

    backDescriptions = []
    for key, value in metadata["Description"]["Value"].items():
        backDescriptions.append({
            'back_id': metadata["Id"],
            'locale': key,
            'string': value
        })
    return backData, backNames, backDescriptions

def upsert(stmt, idx, cols):
    update = dict()
    for c in cols:
        update[c] = stmt.excluded[c]

    return stmt.on_conflict_do_update(
        index_elements=idx,
        set_=update
    )

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--folder', required=True)
    parser.add_argument('--jdbc', required=True)
    args = parser.parse_args()

    engine = create_engine(args.jdbc)
    metadata.create_all(engine)

    allCardData = []
    allCardNames = []
    allCardText = []
    allCardTags = []

    cardFolder = os.path.join(args.folder, 'Cards')
    for cd in os.listdir(cardFolder):
        (card, cn, ct, tags) = handleCardFolder(os.path.join(cardFolder, cd))
        allCardData.append(card)
        allCardNames.extend(cn)
        allCardText.extend(ct)
        allCardTags.extend(tags)

    allCardBacks = []
    allBackNames = []
    allBackDescs = []

    cardBackFolder = os.path.join(args.folder, 'CardBacks')
    for cd in os.listdir(cardBackFolder):
        (back, nms, desc) = handleCardBackFolder(os.path.join(cardBackFolder, cd))
        allCardBacks.append(back)
        allBackNames.extend(nms)
        allBackDescs.extend(desc)

    with engine.begin() as conn:
        conn.execute(upsert(insert(hearthstoneCards).values(allCardData), ['card_id'], ['id', 'has_card', 'has_golden']))
        conn.execute(upsert(insert(hearthstoneCardNames).values(allCardNames), ['card_id', 'locale'], ['string']))
        conn.execute(upsert(insert(hearthstoneCardText).values(allCardText), ['card_id', 'locale'], ['string']))
        conn.execute(upsert(insert(hearthstoneCardTags).values(allCardTags), ['card_id', 'tag'], ['val']))

        conn.execute(upsert(insert(hearthstoneCardBacks).values(allCardBacks), ['id'], ['has_back', 'active']))
        conn.execute(upsert(insert(hearthstoneCardBackNames).values(allBackNames), ['back_id', 'locale'], ['string']))
        conn.execute(upsert(insert(hearthstoneCardBackDescriptions).values(allBackDescs), ['back_id', 'locale'], ['string']))