import matplotlib.pyplot as plt
import matplotlib.axes as ax
import matplotlib.pylab as pylab
import numpy as np

plt.style.use('dark_background')

a = 0.8
b = 2

# make data
x = [0,1,2,3,4,5]
y = [a**(n**b) for n in x]
#x2 = np.linspace(0, 10, 25)
#y2 = 4 + 1 * np.sin(2 * x2)

# plot
#fig, ax = plt.subplots()
plt.plot(x,y, label=f'version_config_score_multiplier = {a} ^ (version_behind ^ {b})')

#ax.plot(x, y, linewidth=2.0)


# naming the x axis
plt.xlabel('Nym Node versions behind the current one', fontsize=24)


# naming the y axis
plt.ylabel('Config score multiplier', fontsize=24)

# giving a title to my graph
plt.title('Nym node version config score multiplier', fontsize=32)


#ax.Axes.set_xticks([x])
#ax.Axes.set_yticks([y])

plt.legend(fontsize=20)

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
plt.show()

#plt.savefig('../docs/public/images/operators/tokenomics/reward_version_graph.png')
