import { FC, useState, useEffect } from 'react'
// import { MixnetQueryClient, settings, PendingEpochEvent } from './client';
// import { CosmWasmClient, SigningCosmWasmClient, ExecuteResult } from "@cosmjs/cosmwasm-stargate";

// const account = 'n1yxdwh3mdslckady4ltxjhcw3v6uu7yrvq7eyxy';
const defaultAccount = 'n167krtck0c9cav7jaet0j50pxlhrtxfj8jjrjg2';
// const account = 'n1few7q29ljc7hunwk9uurukxvzjqhvuna55gy0q';

export const Delegations: FC = () => <div>Delegations</div>;

// export const Delegations: FC = () => {

//     const [account, setAccount] = useState<string>();
//     const [client, setClient] = useState<MixnetQueryClient | null>(null);
//     const [delegations, setDelegations] = useState<any>();
//     const [pendingEvents, setPendingEvents] = useState<PendingEpochEvent[]>([]);
//     const [delegationsTotal, setDelegationsTotal] = useState<number>(0);
//     const [trigger, setTrigger] = useState(0);

//     //Client setup
//     const init = async () => {
//         const cosmWasmClient = await CosmWasmClient.connect(settings.url);
//         const queryClient = new MixnetQueryClient(cosmWasmClient, settings.mixnetContractAddress);
//         setClient(queryClient);
//     };

//     const getDelegations = async () => {
//         if (!client) {
//             return;
//         }
//         let startAfter = undefined;
//         let resp = undefined;
//         const delegations = [];
//         let total = 0;
//         do {
//             resp = await client.getDelegatorDelegations({ delegator: account, startAfter });

//             for (const delegation of resp.delegations) {
//                 const reward = await client.getPendingDelegatorReward({ address: account, mixId: delegation.mix_id });
//                 delegation.reward = reward;
//                 delegation.rewardsNym = Number.parseInt(reward.amount_earned.amount) / 1e6;
//                 total += delegation.rewardsNym
//             }

//             delegations.push(...resp.delegations);

//             startAfter = resp.start_next_after;
//         } while (resp.start_next_after);

//         const newPendingEvents: PendingEpochEvent[] = [];
//         resp = undefined;
//         do {
//             resp = await client.getPendingEpochEvents({ startAfter });
//             newPendingEvents.push(...resp.events);
//             startAfter = resp.start_next_after;
//         } while (resp.start_next_after);

//         setDelegations(delegations);
//         setDelegationsTotal(total);
//         setPendingEvents(newPendingEvents.filter(e => (e.event.kind as any).Delegate?.owner === settings.address));
//         // setPendingEvents(newPendingEvents);

//         delegations.map(async d => {
//             try {
//                 const res = await fetch(`https://explorer.nymtech.net/api/v1/mix-node/${d.mix_id}`);
//                 const mixNode = await res.json();
//                 d.mixNode = mixNode;
//                 setTrigger(prev => prev + 1);
//             } catch (e) {
//                 console.error('Failed to fetch', e);
//             }
//         });

//         delegations.map(async d => {
//             try {
//                 const res = await fetch(`https://explorer.nymtech.net/api/v1/mix-node/${d.mix_id}/description`);
//                 const description = await res.json();
//                 d.description = description;
//                 setTrigger(prev => prev + 1);
//             } catch (e) {
//                 console.error('Failed to fetch', e);
//             }
//         });
//     };

//     useEffect(() => {
//         init();
//     }, []);

//     useEffect(() => {
//         setAccount(settings.address);
//         getDelegations();
//     }, [client]);


//     return (
//         <div style={{padding: '2rem 0'}}>
//             <div>The account {account} has the following delegations:</div>
//             <div>{delegations?.length || 0} delegations</div>
//             <div>{`${Math.floor(delegationsTotal * 100) / 100}`} NYM rewards total</div>
//             <div>
//                 <table>
//                     <thead>
//                         <tr>
//                             <th>Mix Id</th>
//                             <th>Description</th>
//                             <th>Rewards</th>
//                             <th>Saturation</th>
//                             <th>Uptime</th>
//                         </tr>
//                     </thead>
//                     <tbody>
//                         {delegations?.map(d => (
//                             <tr key={`${d.mix_id}`}>
//                                 <td>{d.mix_id}</td>
//                                 <td>{d.description?.name || '-'}</td>
//                                 <td>{Number.parseInt(d.reward.amount_earned.amount) / 1e6}</td>
//                                 <td>{d.mixNode?.stake_saturation ? Math.floor(d.mixNode.stake_saturation * 100) : '-'}</td>
//                                 <td>{d.mixNode?.avg_uptime ? Math.floor(d.mixNode.avg_uptime) : '-'}</td>
//                             </tr>
//                         ))}
//                     </tbody>
//                 </table>
//             </div>
//             {/* <div><pre>{JSON.stringify(pendingEvents, null, 2)}</pre></div> */}
//             <div>
//                 {/* {pendingEvents.map(e => <div>[{e.id}] mixId: {e.event.kind.Delegate.mix_id}, amount: {e.event.kind.Delegate.amount.amount}</div>)} */}
//             </div>
//             {/* <div><pre>{JSON.stringify(delegations, null, 2)}</pre></div> */}
//         </div>
//     )
// };