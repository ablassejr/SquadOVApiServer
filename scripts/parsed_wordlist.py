import argparse
import os
from nltk.corpus import wordnet

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--input', required=True)
    parser.add_argument('--output', required=True)
    args = parser.parse_args()

    nounList = []
    adjectiveList = []
    verbList = []

    with open(args.input, 'r') as iff:
        for line in iff:
            word = ''.join(c for c in line if c.isalnum())
            sets = wordnet.synsets(word)

            if len(sets) == 0:
                continue
            part = sets[0].pos()

            isNoun = part == 'n'
            isAdj = part == 'a'
            isVerb = part == 'v'

            if isNoun and isAdj:
                continue

            if len(word) <= 3 or len(word) > 9:
                continue

            if isNoun:
                nounList.append(word)
            elif isAdj:
                adjectiveList.append(word)
            elif isVerb:
                verbList.append(word)

    with open(os.path.join(args.output, 'nouns.txt'), 'w') as off:
        for n in nounList:
            off.write('{}\n'.format(n))

    with open(os.path.join(args.output, 'adjectives.txt'), 'w') as off:
        for n in adjectiveList:
            off.write('{}\n'.format(n))

    with open(os.path.join(args.output, 'verbs.txt'), 'w') as off:
        for n in verbList:
            off.write('{}\n'.format(n))