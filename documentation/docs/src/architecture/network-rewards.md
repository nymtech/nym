# Network Rewards

Node operator and delegator rewards are determined according to the principles laid out in the section 6 of [Nym Whitepaper](https://nymtech.net/nym-whitepaper.pdf). 

Below is a TLDR of the variables and formulas involved in calculating these rewards per epoch. The initial reward pool contains 250 million NYM, leaving a circulating supply of 750 million NYM.

|Symbol|Definition|
|---|---|
|<img src="https://render.githubusercontent.com/render/math?math=R"></img>|global share of rewards available, starts at 2% of the reward pool. 
|<img src="https://render.githubusercontent.com/render/math?math=R_{i}"></img>|node reward for mixnode `i`.
|<img src="https://render.githubusercontent.com/render/math?math=\sigma_{i}"></img>|ratio of total node stake (node bond + all delegations) to the token circulating supply.
|<img src="https://render.githubusercontent.com/render/math?math=\lambda_{i}"></img>|ratio of stake operator has pledged to their node to the token circulating supply.
|<img src="https://render.githubusercontent.com/render/math?math=\omega_{i}"></img>|fraction of total effort undertaken by node `i`.
|<img src="https://render.githubusercontent.com/render/math?math=k"></img>|number of nodes stakeholders are incentivised to create, set by the validators, a matter of governance. Currently determined by the reward set size.
|<img src="https://render.githubusercontent.com/render/math?math=\alpha"></img>|Sybil attack resistance parameter - the higher this parameter is set the stronger the reduction in competitivness gets for a Sybil attacker.
|<img src="https://render.githubusercontent.com/render/math?math=PM_{i}"></img>|declared profit margin of operator `i`.
|<img src="https://render.githubusercontent.com/render/math?math=PF_{i}"></img>|uptime of node `i`, scaled to 0 - 1, for the rewarding epoch
|<img src="https://render.githubusercontent.com/render/math?math=PP_{i}"></img>|cost of operating node `i` for the duration of the rewarding eopoch.

Node reward for node `i` is determined as:

<img src="https://render.githubusercontent.com/render/math?math=R_{i}=PF_{i} \cdot R \cdot (\sigma^'_{i} \cdot \omega_{i} \cdot k %2b \alpha \cdot \lambda^'_{i} \cdot \sigma^'_{i} \cdot k)/(1 %2b \alpha)"></img>


where:

<img src="https://render.githubusercontent.com/render/math?math=\sigma^'_{i} = min\{\sigma_{i}, 1/k\}"></img>


and

<img src="https://render.githubusercontent.com/render/math?math=\lambda^'_{i} = min\{\lambda_{i}, 1/k\}"></img>


Operator of node `i` is credited with the following amount:

<img src="https://render.githubusercontent.com/render/math?math=min\{PP_{i},R_{i}\} %2b max\{0, (PM_{i} %2b (1 - PM_{i}) \cdot \lambda_{i}/\delta_{i}) \cdot (R_{i} - PP_{i})\}"></img>


Delegate with stake `s` recieves:

<img src="https://render.githubusercontent.com/render/math?math=max\{0, (1-PM_{i}) \cdot (s^'/\sigma_{i}) \cdot (R_{i} - PP_{i})\}"></img>


where `s'` is stake `s` scaled over total token circulating supply.
