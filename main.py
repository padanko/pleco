# めっちゃ複雑なPLECoプログラムを作成する

import random
import string

chars = string.ascii_letters + string.digits

text = "Hello, nice to meet you."


text = list(text)

skips = []
for i in range(len(text)):
    if not text[i] in "\n;":
        mozi = random.randint(20,50)
        text[i] += "".join(random.choices(chars, k=mozi))
        skips.append(mozi)
    else:
        pass
text = ["a"] + text
text.append(";Fff ")
for j in skips:
    text.append(("rf"*j)+"f ")

print("" + "".join(text) + "FvR")