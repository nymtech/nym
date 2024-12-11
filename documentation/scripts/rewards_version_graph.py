import matplotlib.pyplot as plt
import matplotlib.axes as ax
import matplotlib.pylab as pylab
from matplotlib.pyplot import figure
import numpy as np

plt.style.use('dark_background')

a = 0.995
b = 1.65

# make data
x1 = [0,1,2,3,4,5]
x2 = x1
x3 = x1
x4 = x1

y1 = [a**((v*1)**b) for v in x1]
y2 = [a**((v*10)**b) for v in x1]
y3 = [a**((v*100)**b) for v in x1]
# y4 = [a**((11)**b) for v in x1]

f = plt.figure()
f.set_figwidth(12)
f.set_figheight(9)

# plot
#fig, ax = plt.subplots()
plt.plot(x1,y1, label=f'Patches behind:             config_score_multiplier = {a} ^ ((1 * versions_behind) ^ {b})')
plt.plot(x2,y2, label=f'Minor versions behind:  config_score_multiplier = {a} ^ ((10 * versions_behind) ^ {b})')
plt.plot(x3,y3, label=f'Major versions behind:  config_score_multiplier = {a} ^ ((100 * versions_behind) ^ {b})')
#ax.plot(x, y, linewidth=2.0)


# naming the x axis
plt.xlabel('Nym Node versions behind the current one', fontsize=20)


# naming the y axis
plt.ylabel('Config score multiplier', fontsize=20)

# giving a title to my graph
plt.title('Nym node version config score multiplier', fontsize=28)


#ax.Axes.set_xticks([x])
#ax.Axes.set_yticks([y])

plt.legend(fontsize=12)

#params = {'legend.fontsize': 20,
#         'axes.labelsize': 24,
#         'axes.titlesize':'x-large',
#         'xtick.labelsize':20,
#         'ytick.labelsize':20}
#
#pylab.rcParams.update(params)

# set the limits
plt.xlim([0, 5])
plt.ylim([0,1])



#plt.show()

plt.savefig('../docs/public/images/operators/tokenomics/reward_version_graph.png')
