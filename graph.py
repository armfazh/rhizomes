import json
import matplotlib.pyplot as plt
import numpy as np
import sys

if len(sys.argv) == 4:
    input0 = sys.argv[1]
    input1 = sys.argv[2]
    output = sys.argv[3]
else:
    print("Usage:\n\tpython3 plot.py <first.json> <second.json> <image.png>")

with open(input0) as file:
    dataset0 = json.load(file)
with open(input1) as file:
    dataset1 = json.load(file)

name0 = dataset0["name"]
name1 = dataset1["name"]
cat0 = [n for n in dataset0["benchmarks"]]
cat1 = [n for n in dataset1["benchmarks"]]
categories = sorted(list(set(cat0) & set(cat1)))[::-1]

values0 = []
values1 = []
for c in categories:
    t0 = dataset0["benchmarks"][c]["criterion_estimates_v1"]["mean"]["point_estimate"]
    t1 = dataset1["benchmarks"][c]["criterion_estimates_v1"]["mean"]["point_estimate"]
    values0 += [t0 / t0]
    values1 += [t0 / t1]

y_pos = np.arange(len(categories))
height = 0.3
plt.barh(y_pos - height / 2, values0, height=height, label=name0)
plt.barh(y_pos + height / 2, values1, height=height, label=name1)
plt.xlabel("Speedup")
plt.title(f"Speedup {name0} vs {name1}")
plt.yticks(y_pos, categories)
plt.legend()
fig = plt.gcf()
fig.set_size_inches(10, 12)
fig.tight_layout()
fig.savefig(output, dpi=200)