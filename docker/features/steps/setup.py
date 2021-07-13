from behave import *
import docker
import time

VALIDATOR_PATH = "validator"
WALLET_CLIENT = "web_wallet"
VALIDATOR_TAG = "validator:latest"
CLIENT_TAG = "client:latest"


@given('Docker images are built')
def step_impl(context):
    context.client = docker.from_env()
    context.client.images.build(path=VALIDATOR_PATH, tag=VALIDATOR_TAG)
    context.client.images.build(path=WALLET_CLIENT, tag=CLIENT_TAG)


@when('{amount:d} validators are up and running')
def step_impl(context, amount):
    assert amount > 0
    context.volume = context.client.volumes.create("common_volume")
    context.network = context.client.networks.create("common_network")
    context.genesis = context.client.containers.run(image=VALIDATOR_TAG, command="genesis", name="docker_genesis_validator_1", detach=True, volumes=["common_volume:/genesis_volume"], network="common_network")
    context.secondaries = []
    for _ in range (amount - 1):
        secondary = context.client.containers.run(VALIDATOR_TAG, "secondary", detach=True, volumes=["common_volume:/genesis_volume"], network="common_network")
        context.secondaries.append(secondary)
    time.sleep(10)


@then('upload contract')
def step_impl(context):
    context.wallet_client = context.client.containers.run(CLIENT_TAG, detach=True, volumes=["common_volume:/genesis_volume"], network="common_network") 
    assert context.wallet_client.wait()['StatusCode'] == 0

@then('cleanup environment')
def step_impl(context):
    context.genesis.remove(force=True)
    for secondary in context.secondaries:
        secondary.remove(force=True)
    context.wallet_client.remove(force=True)
    context.volume.remove()
    context.network.remove()
