<script>
  import {invoke} from "@tauri-apps/api/tauri";
  import {onMount} from "svelte";
  import QRious from "qrious";

  const validator_urls = ["http://localhost:8080", "http://localhost:8081", "http://localhost:8082"];
  let signatures = [];
  let qrVisible = false;

  async function getCredential() {
    signatures = await invoke("get_credential", {
      validatorUrls: validator_urls,
    });
  }

  async function randomiseCredential(idx) {
    signatures = await invoke("randomise_credential", {
      idx: idx,
    });
  }

  async function verifyCredential(idx) {
    const response = await invoke("verify_credential", {
      idx: idx,
      validatorUrls: validator_urls,
    });
    qrVisible = !response;
  }

  async function deleteCredential(idx) {
    signatures = await invoke("delete_credential", {
      idx: idx,
    });
  }

  async function listCredentials() {
    signatures = await invoke("list_credentials");
  }

  function signatureQR(idx) {
    qrVisible = true;
    const signature = signatures[idx];
    new QRious({
      element: document.getElementById("qr"),
      value: signature,
      foreground: "white",
      background: "black",
      size: 148,
    });
  }

  onMount(() => {
    listCredentials();
  });
</script>

<svelte:head>
  <title>Coconut</title>
</svelte:head>

<button class="btn btn-success" on:click={getCredential}>Get Credential</button>
<hr />
<table class="table table-dark">
  {#each signatures as signature, idx}
    <tr>
      <td><p>{signature.slice(0, 12)}</p></td>
      <td>
        <div class="btn-group" role="group" aria-label="Basic example">
          <button
            class="btn btn-primary"
            on:click={() => {
              randomiseCredential(idx);
            }}>Randomize</button
          ><button
            class="btn btn-danger"
            on:click={() => {
              deleteCredential(idx);
            }}>Delete</button
          >
          <button
            class="btn btn-info"
            on:click={() => {
              signatureQR(idx);
            }}>QR</button
          >
          <button
            class="btn btn-primary"
            on:click={() => {
              verifyCredential(idx);
            }}>Verify</button
          >
        </div></td
      >
    </tr>
  {/each}
</table>

<div
  class="modal"
  tabindex="-1"
  style={qrVisible ? "display: block" : "display: none"}
>
  <div class="modal-dialog modal-sm">
    <div class="modal-content">
      <div class="modal-body">
        <canvas id="qr"></canvas>
        <button
          type="button"
          class="close"
          on:click={() => (qrVisible = false)}
        >
          <span aria-hidden="true">&times;</span>
        </button>
      </div>
    </div>
  </div>
</div>

<!-- <div><br /></div> -->
