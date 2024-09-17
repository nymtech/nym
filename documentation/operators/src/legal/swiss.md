# Legal environment: Switzerland

```admonish info
The entire content of this page is under [Creative Commons Attribution 4.0 International Public License](https://creativecommons.org/licenses/by/4.0/).
```

```admonish warning
The following part is for informational purposes only. Nym core team cannot provide comprehensive legal advice across all jurisdictions. Knowledge and experience with the legalities are being built up with the help of our counsel and with you, the community of Nym node operators. We encourage Nym node operators to join the [Node Operator](https://matrix.to/#/#operators:nymtech.chat) and [Operators Legal Forum](https://matrix.to/#/!YfoUFsJjsXbWmijbPG:nymtech.chat?via=nymtech.chat&via=matrix.org) channels on Element to share best practices and experiences.
```

## Findings from our legal team

> **Note:** The information shared below is in the stage of conclusions upon final confirmation. The text is a not edited exert from a legal counsel. Nym core team is asking for more clarifications. 

### Operators of Exit Nodes

#### Telecoms Law

As well as operators of normal mixnet nodes, operators of exit nodes might be considered telecommunications providers according to the broad term of the telecommunications act (TCA).
The regulatory consequences have already been laid out in section 5.1.2.2.1 above.

#### Telecoms Surveillance Law

Unlike normal mixnet nodes, exit nodes might have information about the communication party which uses the respective exit node (in particular its IP address). They might therefore be a target for surveillance authorities, at least at first glance.

However, as the IP address of the communications party is disguised on the other side of the communications through the Nym encryption infrastructure, the usual situation, where an IP address or another trace of an Internet user is found in the connection with a criminal activity (e.g., in a web server protocol), and then used in cooperation with the user’s provider to identify the user, is not going to take place.
 
The same is true for the opposite side: The node operator does not see the communication party of his user.

Experience has shown that Swiss investigative authorities are aware of these limitations and do not conduct investigations against individuals who operate TOR nodes, for example. In one specific case that I know of, the investigation was stopped by the police as soon as it was clear that a TOR node was being operated.

I therefore consider the risk for an exit node operator to become involved in a SPTA proceeding as low.

Nevertheless, in such a situation, exit node operators providers would have to provide the authorities with the information already available to them (Art. 22 Para. 3 SPTA), and they would have to tolerate monitoring by the authorities or by the persons commissioned by the service of the data which the monitored person transmits or stores using derived communications services (Art. 27 Para. 1 SPTA; see above, 5.1.1.2). There is no duty of data retention for providers of derived communication services, though.

The risk for exit node operators of being upgraded according to Art. 22 Para. 4 SPTA is low to non existent for the reasons mentioned above.

#### Intelligence Service Law

Operators of exit nodes do not provide wire-based telecommunications services either and therefore do not fall under the IntelSA.

### Nym as VPN provider

#### Telecoms Law

Nym as a VPN operator might be considered a telecommunication provider under the newly revised TCA, as the term now also covers operators of Over-the-Top services which are carried out over the internet. 

However I consider possible administrative burdens arising from this qualification as negligible (see above, 5.1.2.1).

#### Telecoms Surveillance Law

VPN providers have information about the communication party which uses the respective exit node (in particular its IP address). They might therefore be a target for surveillance authorities, at least at first glance.

However, for the same reason I see a risk low for exit node operators to become involved in a SPTA proceeding (the IP address is not visible to the communication partner, which is exactly the reason the Nym VPN is being used at all), I also see a low risk for Nym itself to become involved in such a proceeding (see above, 5.1.3.2).

#### Intelligence Service Law

VPN operators do not provide wire-based telecommunications services and therefore do not fall under the IntelSA.

### EU chat control regulation in particular

According to a EU commission proposal for a regulation laying down rules to prevent and combat child sexual abuse (https://eur-lex.europa.eu/legal-content/EN/TXT/HTML/?uri=CELEX: 52022PC0209) hosting providers and providers of so-called interpersonal communication services should be obliged to perform an assessment of risks of online child sexual abuse. Additionally an obligation for certain providers should be established to detect such abuse, to report it via the EU Centre, to remove or disable access to, or to block online child sexual abuse material when so ordered.

'Interpersonal communications service’ means a service normally provided for remuneration that enables direct interpersonal and interactive exchange of information via electronic communications networks between a finite number of persons, whereby the persons initiating or participating in the communication determine its recipient(s) and does not include services which enable interpersonal and interactive communication merely as a minor ancillary feature that is intrinsically linked to another service (Art. 2 Point 5 Directive (EU) 2018/1972, which is also relevant for the mentioned proposal).

Interpersonal communications services are services that enable interpersonal and interactive exchange of information. Interactive communication entails that the service allows the recipient of the information to respond. The proposal therefore only covers services like traditional voice calls between two individuals but also all types of emails, messaging services, or group chats. Examples for services which do not meet those requirements are linear broadcasting, video on demand, websites, social networks, blogs, or exchange of information between machines (Directive (EU) 2018/1972, Consideration 17).

Neither the Nym encryption infrastructure nor the NYM VPN are used as means for an interactive exchange of information in the aforementioned sense (of e-mail, messaging, chats or similar).
 
I therefore consider the risk arising from the mentioned proposal for Nym as low, be it as software developer or VPN operator. 

However, an application provider which uses the Nym encryption infrastructure to provide encrypted chat services or similar could still fall under the proposal. This might pose a commercial risk for Nym as the provider of the basic infrastructure for such services, because such services might lose their commercial value for end customers.

Currently the EU decision on chat control has been postponed because there is a blocking minority which can prevent the adoption of the respective parts of the law. In addition, even EU internal lawyers held that the proposal was clearly in violation of the EU charter of fundamental rights and would therefore be nullified by the EU courts in case it would still be enacted by the parliament. 

I therefore consider the risk that the mentioned proposal is enacted by the EU authorities and finally upheld by the courts in its planned form as low.

