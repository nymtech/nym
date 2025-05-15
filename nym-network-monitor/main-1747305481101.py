import csv
import arrow


class Measurement:
    def __init__(self, serial, ts, route, received): #, tstamp):
        self.serial = serial  # serial number identifying the packet (ordered in time)
        self.ts = ts  # timestamp
        self.route = route  # route of the packet in the form [node_L1, node_L2, node_L3, gateway]
        self.received = received  # True if received, False if not
        self.duplicates = 0  # increases with the number of duplicates


class Node:
    def __init__(self, node_id):
        self.node_id = node_id  # serial number identifying the node
        self.mix = False  # it's set to true when the node is seen in a mix layer position
        self.gateway = False  # it's set to true when the node is seen in a gateway position
        self.pos_samples = 0  # measurement samples routed by this node and successfully received
        self.neg_samples = 0  # measurement samples considered as possibly dropped by this node
        self.fail_seq = 0  # nr of packets dropped in a sequence
        self.pos_samples_v2 = 0  # measurement samples routed by this node and successfully received (current system)
        self.neg_samples_v2 = 0  # measurement samples considered as possibly dropped by this node (current system)
        self.score = 0  # performance score computed after new postprocessing
        self.score_v2 = 0  # current performance score


def read_input_file(file_name):

    list_measurements = []
    dict_nodes = {}
    with open(file_name, newline='') as csvfile:
        reader = csv.reader(csvfile, delimiter=',', quotechar='|')
        for row in reader:
            if row[0] == 'id':
                continue
            serial = int(row[0])
            ts = arrow.get(row[1])
            ts_seconds = ts.timestamp()
            n1 = int(row[2])
            n2 = int(row[3])
            n3 = int(row[4])
            gw = int(row[5])
            if row[6] == 'false':
                received = False
            elif row[6] == 'true':
                received = True
            else:
                exit('ERROR booloan from list, value = ' + str('row[6]'))

            if len(list_measurements) > 0:
                last_msm = list_measurements[-1]
                if n1 == last_msm.route[0] and n2 == last_msm.route[1] and n3 == last_msm.route[2] and gw == last_msm.route[3]:
                    last_msm.duplicates += 1
                else:
                    msm = Measurement(serial, ts_seconds, [n1, n2, n3, gw], received)
                    list_measurements.append(msm)
            else:
                msm = Measurement(serial, ts_seconds, [n1, n2, n3, gw], received)
                list_measurements.append(msm)

            for n in [n1, n2, n3, gw]:
                if n not in dict_nodes.keys():
                    dict_nodes[n] = Node(n)

    return list_measurements, dict_nodes


def compute_node_scores(list_measurements, dict_nodes):

    for msg in list_measurements:
        if msg.received:
            for node in msg.route:
                dict_nodes[node].pos_samples += 1  # count measurement as positive
                dict_nodes[node].fail_seq = 0  # reset sequence of failed messages
                dict_nodes[node].pos_samples_v2 += 1  # count measurement as positive
        else:  # message was dropped
            guilty = []  # candidates for being responsible for the drop
            for node in msg.route:
                dict_nodes[node].fail_seq += 1
                dict_nodes[node].neg_samples_v2 += 1  # count measurement as negative
                if dict_nodes[node].fail_seq > 2:
                    guilty.append(node)
                    dict_nodes[node].neg_samples += 1
            if len(guilty) == 0:  # none of them is obviously dropping packets
                for node in msg.route:
                    dict_nodes[node].neg_samples += 1  # punish all three with a negative sample

    for node in dict_nodes.values():
        if node.pos_samples + node.neg_samples == 0:
            exit("not enough samples causes division by zero")

        estimated_performance = node.pos_samples / (node.pos_samples + node.neg_samples)
        estimated_performance_v2 = node.pos_samples_v2 / (node.pos_samples_v2 + node.neg_samples_v2)
        print("\nnode id: ", node.node_id)
        print("estimated performance (new v2.5): ", round(estimated_performance, 2))
        print("estimated performance (current v2): ", round(estimated_performance_v2, 2))


if __name__ == '__main__':

    file_name = 'routes_v0.csv'
    list_measurements, dict_nodes = read_input_file(file_name)
    compute_node_scores(list_measurements, dict_nodes)