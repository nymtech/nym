<script>
  import { invoke } from "@tauri-apps/api/tauri";
  import { onMount } from "svelte";
  import QRious from "qrious";

  const validator_urls = ["http://localhost:8080"];
  let signatures = [];
  let qrVisible = false;

  async function getCredential() {
    const response = await invoke("get_credential", {
      validatorUrls: validator_urls,
    });
    signatures = response;
    // console.log(signature);
  }

  async function randomiseCredential(idx) {
    const response = await invoke("randomise_credential", {
      idx: idx,
    });
    signatures = response;
    // console.log(signature);
  }

  async function deleteCredential(idx) {
    const response = await invoke("delete_credential", {
      idx: idx,
    });
    signatures = response;
    // console.log(signature);
  }

  async function listCredentials() {
    const response = await invoke("list_credentials");
    signatures = response;
    // console.log(signature);
  }

  function signatureQR(idx) {
    qrVisible = true;
    const signature = signatures[idx];
    const qr = new QRious({
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
    <div class="modal-content" >
      <div class="modal-body">
        <canvas id="qr" />
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
