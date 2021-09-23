import argparse
import os
import csv
import json
import shutil
from sqlalchemy import create_engine, Table, Column, Integer, String, MetaData, Boolean, ForeignKey
from sqlalchemy.sql import text
from sqlalchemy.dialects.postgresql import insert

metadata = MetaData()

mappingTable = Table('wow_spell_to_class', metadata,
    Column('build_id', String, nullable=False),
    Column('spell_id', Integer, nullable=False),
    Column('class_id', Integer, nullable=False),
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

def setToClass(set):
    if set == 3:
        return 8 # mage
    elif set == 4:
        return 1 # warrior
    elif set == 5:
        return 9 # warlock
    elif set == 6:
        return 5 # priest
    elif set == 7:
        return 11 # druid
    elif set == 8:
        return 4 # rogue
    elif set == 9:
        return 3 # hunter
    elif set == 10:
        return 2 # paladin
    elif set == 11:
        return 7 # shaman
    return 0

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--data', required=True)
    parser.add_argument('--jdbc', required=True)
    parser.add_argument('--build', required=True)
    args = parser.parse_args()

    allMappingData = []
    with open(os.path.join(args.data)) as mappings:
        reader = csv.DictReader(mappings)
        for row in reader:
            if 'SpellID' not in row:
                continue

            spellId = int(row['SpellID'])
            spellClass = setToClass(int(row['SpellClassSet']))
            if spellClass == 0:
                continue
            allMappingData.append({
                'build_id': args.build,
                'spell_id': spellId,
                'class_id': spellClass,
            })

    engine = create_engine(args.jdbc)
    metadata.create_all(engine)
    with engine.begin() as conn:
        conn.execute(upsert(insert(mappingTable).values(allMappingData), ['build_id', 'spell_id'], ['class_id']))

if __name__ == '__main__':
    main()