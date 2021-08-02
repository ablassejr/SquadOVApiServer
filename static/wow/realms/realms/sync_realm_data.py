import argparse
from sqlalchemy import create_engine, Table, Column, Integer, String, MetaData, Boolean, ForeignKey
from sqlalchemy.sql import text
from sqlalchemy.dialects.postgresql import insert
import os
import json

metadata = MetaData()

wowRealms = Table('wow_realms', metadata,
    Column('id', Integer, primary_key=True),
    Column('name', String, nullable=False),
    Column('slug', String, nullable=False),
    Column('region', String, nullable=False),
    schema='squadov',
)

def upsert(stmt, idx, cols):
    update = dict()
    for c in cols:
        update[c] = stmt.excluded[c]

    return stmt.on_conflict_do_update(
        index_elements=idx,
        set_=update
    )

regions = ['us', 'eu', 'kr', 'tw']

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--realms', required=True)
    parser.add_argument('--jdbc', required=True)
    args = parser.parse_args()

    allRealms = []
    for reg in regions:
        with open(os.path.join(args.realms, '{}.json'.format(reg)), encoding='utf-8') as f:
            realms = json.load(f)

        for r in realms["realms"]:
            allRealms.append({
                'id': r["id"],
                'name': r["name"],
                'slug': r["slug"],
                'region': reg
            })

    engine = create_engine(args.jdbc)
    metadata.create_all(engine)
    with engine.begin() as conn:
        conn.execute(upsert(insert(wowRealms).values(allRealms), ['id'], ['id', 'name', 'slug', 'region']))