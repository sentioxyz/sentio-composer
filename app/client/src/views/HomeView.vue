<template>
  <h1>Call Aptos Function</h1>
  <main>
    <form @submit.prevent="callFunction" class="mb-3">
      <div v-if="error" class="alert alert-dismissible alert-warning">
        <button type="button" class="close" data-dismiss="alert">×</button>
        <h4 class="alert-heading">Error!</h4>
        <p class="mb-0">{{ error }}</p>
      </div>
      <div class="form-group">
        <label for="function">Function</label>
        <textarea
          v-model="message.func"
          style="height: 4rem"
          type="text"
          class="form-control"
          id="function"
          placeholder="Enter a function name, e.g. 0x1::foo::bar"
          required
        ></textarea>
      </div>
      <div class="form-group">
        <label for="type_args">Type parameters</label>
        <textarea
          v-model="message.type_args"
          type="text"
          style="height: 4rem"
          class="form-control"
          id="type_args"
          placeholder="Enter type parameters, seperated by ','"
        ></textarea>
      </div>
      <div class="form-group">
        <label for="ledger_version">Ledger Version</label>
        <input
          v-model="message.ledger_version"
          type="text"
          class="form-control"
          id="ledger_version"
          placeholder="Enter the ledger version"
        />
      </div>
      <div class="form-group">
        <label for="network">Network</label>
        <input
          v-model="message.network"
          type="text"
          class="form-control"
          id="network"
          placeholder="Enter Network"
        />
      </div>
      <div class="form-group">
        <label for="args">Parameters</label>
        <textarea
          v-model="message.args"
          type="text"
          class="form-control"
          style="height: 4rem"
          id="args"
          placeholder="Enter parameters, seperated by ','"
        ></textarea>
      </div>
      <div style="text-align: right; margin-top: 1rem">
        <button type="submit" class="btn btn-primary">Call Function</button>
      </div>
    </form>
    <div>
      <label for="result">Result</label>
      <textarea
        v-show="isShow"
        readonly
        v-model="result"
        type="text"
        style="height: 15rem"
        id="result"
      ></textarea>
    </div>
  </main>
</template>

<style>
h1 {
  font-size: 1rem;
}

main {
  background: white;
  border-radius: 0.25rem;
  padding: 1rem 2rem 2rem;
  display: grid;
  grid-gap: 1rem;
  line-height: 1.25rem;
}

@media (min-width: 50rem) {
  main {
    grid-gap: 3rem;
    grid-template-columns: 1fr 1fr;
  }
}

label {
  display: block;
  margin: 1rem 0 0.25rem;
}

img {
  max-width: 300px;
  height: auto;
}

input,
textarea {
  width: 100%;
  padding: 0.5rem;
  border: 1px solid #d1d5db;
  border-radius: 0.25rem;
  line-height: inherit;
}

input:focus,
textarea:focus {
  outline: none;
  border-color: #42b983;
}

.btn.btn-primary {
  line-height: inherit;
  padding: 0.5rem 1rem;
  background: #42b983;
  border: none;
  color: white;
  border-radius: 0.25rem;
  cursor: pointer;
}
</style>

<script lang="ts">
import { defineComponent } from 'vue';

const API_URL = 'http://localhost:4000/call_function';

export default defineComponent({
  name: 'sentio-homepage',
  data: () => ({
    error: '',
    // messages: [],
    message: {
      func: '',
      type_args: '',
      args: '',
      ledger_version: 0,
      network: '',
    },
    isShow: false,
    result: '',
  }),
  computed: {
    // reversedMessages() {
    //   return this.messages.slice().reverse();
    // }
  },
  // mounted() {
  //   this.resize();
  // },
  methods: {
    callFunction() {
      console.log(this.message);
      fetch(API_URL, {
        method: 'POST',
        body: JSON.stringify(this.message),
        headers: {
          'content-type': 'application/json',
        },
      })
        .then((response) => response.json())
        .then((result) => {
          if (result.error) {
            // there was an error...
            const error = 'Failed to call the function, check logs in the result';
            this.error = error;
            this.result = result.details;
          } else {
            this.error = '';
            this.result = result.details;
          }
          this.isShow = true;
        });
    },
  },
});
</script>
