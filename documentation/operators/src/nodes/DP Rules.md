## Nym Delegation Program Rules
## July 2024
## Introduction
Welcome to the Nym Delegations Program! As part of this initiative, a portion of the Nym Treasury is delegated (staked) on community-operated Nym nodes. The aims of the Nym Delegation Program are: 
-  To support outstanding Nym node operators by enabling their nodes to become profitable faster; 
-  To improve the quality of service, decentralization, and geographical spread of the Nym network; 
-  To generate income for the Squad Wealth Fund, the Nym community’s main treasury which provides funding for community initiatives and contributions.
By applying to the Program, you accept its rules. The rules below govern the Nym Delegations Program and apply universally to all individuals/organizations who apply and participate in the Program.  

## General rules

   1. All communications and important announcements take place on Nym’s Matrix server. Operators are expected to register an account with Matrix and follow Nym’s relevant channels on Matrix: [announcements channel](https://matrix.to/#/%23node-ops-announcements:nymtech.chat) [node operators chat](https://matrix.to/#/%23operators:nymtech.chat)
  2. These rules are subject to change.  Changes will always be announced on the Matrix channels listed above. Please make sure you follow Matrix channels above regularly for the latest updates. Not following the latest update may result in losing your delegation.  

## Server Requirements
   3. Every server must meet the following minimum requirements:
        a) 4x CPU, 4GB RAM, 40GB SSD; 
        b) both IPv4 and IPv6 must be supported and correctly configured;
        c) 1Tb of monthly traffic at 1Gbps speed.
   4. Every node must have the following running parameters:
        a) Maximum profit margin (PM) – 20%;
        b) Maximum operator cost (OC) – 800 NYM;
        c) Maximum saturation – 50%;
        d) Minimum average routing score – 90%.
        e) Operator must accept Terms & Conditions – see Rule 11 below;
        f) Hardware port must be open via the following flags set to true (default):
            1. --expose-system-info 
            2. --expose-system-hardware 
            3. --expose-crypto-hardware 
        g) Node must be run on default [ports](https://nymtech.net/operators/nodes/maintenance.html#ports)
		h) Accepting the operator terms and conditions is mandatory to enter the Delegations Program (and for your node to be selected to the active set). Starting from version 1.1.3, you must add the following [flag](https://nymtech.net/operators/nodes/setup.html#terms--conditions) to the service file accepting terms & conditions of running a Nym node. You can read more about it [here](https://nymtech.net/operators/toc.html).

## Exclusive use of server

5. It is expected that the server used to host a Nym node will be exclusively used by the Nym node alone.
6. Please make sure you do not host other apps/nodes on the server.

## Prioritization

7. Depending on delegation size and the number of NYM tokens allocated, the Delegations Program has a limited number of slots available. For this reason, the program operates a queue system. This means that by signing up for the Delegations Program, you join a queue and once a free slot becomes available, your node receives delegations automatically. By default, the queue is “first come, first served”. In other words, nodes are queued in order of registration. However, Mentors may prioritize/de-prioritise nodes based on certain criteria and characteristics:
    a) Certain countries/regions may be prioritized, depending on current network requirements;
    b) Nodes operated by Nym community contributors and squads may be prioritized;
    c) Nodes operated for social good may be prioritized. For example, if the NYM tokens received from running a node are donated to charity.
    d) Nodes operated on robust hardware may be prioritized;
    e) Countries with a high number of existing Nym nodes may be de-prioritized.
    f) Nodes running on popular VPS providers may be de-prioritized. For example, popular providers are Hetzner, Contabo, PQ.Hosting, Vultr, OVH. The aim of this Rule is to stimulate community to run Nym nodes on different VPS providers. There is a community maintained [list](https://nymtech.net/operators/legal/isp-list.html) of providers and everyone is welcome to contribute to the list.
    g) If you run more than one node and those nodes are already part of the Nym Delegations Program, the delegation from such node(s) may be withdrawn in favour of other operators who do not have a participating node yet. If this happens, you will be contacted on Matrix. 

## Binary updates

 8. Please make sure to only download binary updates from the official [Github](https://github.com/nymtech/nym/releases).
 9. It is essential to keep the binary’s version up-to-date. The binary your node is running cannot be older than two releases. For example, as of now the latest version is 1.1.5, it means that versions 1.1.4 and 1.1.3 are allowed, version 1.1.2 is **not** allowed.
 10. A node with outdated binaries may lose its delegation at any time without prior notice.

## Re-registering

11. If you lost the delegation or your registration was invalid, you may always re-apply at any time by notifying a Mentor on the Matrix [channel](https://matrix.to/#/%23operators:nymtech.chat). If the problem with your node has been fixed, it will be allowed to join the queue again.
12. You will have to fix the issues and your node will be re-added to the queue.
13. Dishonest/malicious behaviour and repeated poor performance will result in a ban. Once your node is banned, it cannot join the queue again. 



## Reasons for delegation withdrawal

14. The delegation may immediately be withdrawn if the following will occur:
        a) The server will be offline for longer than XX hours consequently, or offline for shorter periods repeatedly;Server’s networking is underperforming;
        b) Server’s networking is underperforming;
        c) Node no longer meets the minimum specification outlined above;
        d) Node’s operating parameters are changed and  fall outside of the requirements outlined above;
        e) Unfair, dishonest or malicious behaviour is detected, such as multi accounting, any attempt to get an unfair advantage over other operators, or exploiting delegators;
		f) In case Mentors advise you about an issue with your node, please make sure you act swiftly and be responsive. Failing to meet deadlines may result in losing your delegations, especially if mentors cannot reach you on Matrix for an extended period of time.

## List of Mentors

-    Vinlexnodes
-    Merve
-    John Smith
-    Rocio.Gonzalez.Toral
-    Noisk8