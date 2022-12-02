<template>
  <h1>Call Aptos Function</h1>
  <main>
    <form @submit.prevent="callFunction" @reset.prevent="clearInput" class="mb-3">
      <div v-if="error" class="alert alert-dismissible alert-warning">
        <button type="button" class="close" data-dismiss="alert">×</button>
        <h4 class="alert-heading">Error!</h4>
        <p class="mb-0">{{ error }}</p>
      </div>
      <div class="form-group">
        <label for="function">Function</label>
        <textarea
          v-model="message.func"
          style="height: 4rem;"
          type="text"
          class="form-control"
          id="function"
          placeholder="Enter a qualified function name, e.g. 0x1::coin::balance"
          required
        ></textarea>
      </div>
      <div class="form-group">
        <label for="type_args">Type arguments</label>
        <textarea
          v-model="message.type_args"
          type="text"
          style="height: 4rem"
          class="form-control"
          id="type_args"
          placeholder="Enter type arguments, one parameter per line"
        ></textarea>
      </div>
      <div class="form-group">
        <label for="args">Arguments</label>
        <textarea
          v-model="message.args"
          type="text"
          class="form-control"
          style="height: 4rem"
          id="args"
          placeholder="Enter arguments, one parameter per line"
        ></textarea>
      </div>
      <div class="form-group">
        <label for="ledger_version">Ledger Version(0 means latest)</label>
        <input
          v-model="message.ledger_version"
          type="text"
          class="form-control"
          id="ledger_version"
          placeholder="Enter a ledger version"
        />
      </div>
      <div class="form-group">
        <label for="network">Network</label>
        <select
          v-model="message.network"
          id="network"
          style="height: 2rem"
        >
          <option disabled value="">Please select network</option>
          <option>mainnet</option>
          <option>testnet</option>
          <option>devnet</option>
        </select>
      </div>
      <div style="text-align: left;">
        <label style="display: inline-block;" for="checkbox">Enable debug logs</label>
        <input style="display: inline-block; margin-left: -10rem;"
        type="checkbox" id="checkbox" v-model="message.with_logs" />
      </div>
      <div class="examples">
        <button class="btn example" type="button" @click="() => loadExample(0)">Example 1</button>
        <button class="btn example" type="button" @click="() => loadExample(1)">Example 2</button>
        <button class="btn example" type="button" @click="() => loadExample(2)">Example 3</button>
      </div>
      <div style="text-align: right; margin-top: 1rem">
        <button
          style="display: inline-block; margin-right: 1rem;"
          type="reset"
          class="btn btn-reset">
          Clear inputs
        </button>
        <button style="display: inline-block;" type="submit" class="btn btn-primary">
          Call Function
        </button>
      </div>
    </form>
    <div>
      <label for="result">Result</label>
      <textarea
        v-show="isShow"
        readonly
        v-model="result"
        type="text"
        style="height: 21rem"
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
  border-color: #253E5D;
}

.btn {
  line-height: inherit;
  padding: 0.5rem 1rem;
  border: none;
  border-radius: 0.25rem;
  cursor: pointer;
}

.btn.btn-primary {
  background: #20B2E4;
  color: white;
}

.btn.btn-reset {
  background: #FE75B4;
  color: white;
}

.examples {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  grid-gap: 1rem;
}

.example {
  margin: 1rem 0;
  padding: 1rem;
  background: #f8fafc;
  border-radius: 4px;
}

.example:hover {
  background: #20B2E4;
  color: white;
}
</style>

<script lang="ts">
import { defineComponent } from 'vue';
import axios from 'axios';

export default defineComponent({
  name: 'sentio-homepage',
  data: () => ({
    error: '',
    message: {
      func: '0x1::coin::balance',
      type_args: '0x1::aptos_coin::AptosCoin',
      args: '0x21ddba785f3ae9c6f03664ab07e9ad83595a0fa5ca556cec2b9d9e7100db0f07',
      ledger_version: 35842267,
      network: 'mainnet',
      with_logs: false,
    },
    isShow: false,
    result: '',
  }),
  watch: {
    $route: {
      immediate: true,
      handler(to, from) {
        document.title = to.meta.title || 'Sentio Composer';
      },
    },
    message: {
      handler(newValue) {
        this.$router.replace({
          query: newValue,
        });
      },
      deep: true,
    },
  },
  computed: {
  },
  mounted() {
    const { query } = this.$route;
    if (query.func) {
      this.message = {
        ...query,
        with_logs: query.with_logs === 'true',
      };
    }
  },
  methods: {
    callFunction() {
      console.log(this.message);
      const typeArgs = this.message.type_args.trim();
      const args = this.message.args.trim();
      const network = this.message.network.trim();
      const requestBody = {
        func: this.message.func,
        type_args: typeArgs.length > 0 ? typeArgs.split('\n') : undefined,
        args: args.length > 0 ? args.split('\n') : undefined,
        ledger_version: this.message.ledger_version,
        network: network.length > 0 ? network : undefined,
        options: this.message.with_logs ? { with_logs: true } : undefined,
      };
      axios.post('/api/call_function', requestBody)
        .then((response) => {
          const result = response.data;
          if (result.error) {
            // there was an error...
            const error = 'Failed to call the function, check errors in the result or re-call the function with debug logs enabled';
            this.error = error;
            this.result = JSON.stringify(result.details, null, 2);
          } else {
            this.error = '';
            if (!this.message.with_logs) {
              this.result = result.details.return_values.join('\n');
            } else {
              this.result = JSON.stringify(result.details, null, 2);
            }
          }
          this.isShow = true;
        });
    },
    clearInput() {
      this.message = {
        func: '',
        type_args: '',
        args: '',
        ledger_version: 0,
        network: '',
        with_logs: false,
      };
    },
    loadExample(n) {
      const examples = [
        {
          func: '0x1::coin::balance',
          type_args: '0x5a97986a9d031c4567e15b797be516910cfcb4156312482efc6a19c0a30c948::lp_coin::LP<0x1::aptos_coin::AptosCoin, 0x84d7aeef42d38a5ffc3ccef853e1b82e4958659d16a7de736a29c55fbbeb0114::staked_aptos_coin::StakedAptosCoin, 0x190d44266241744264b964a37b8f09863167a12d3e70cda39376cfb4e3561e12::curves::Stable>',
          args: '0xc0cbc38f05f1aed5467b889ef704135bc01f0a492d6fd28ef499d19e7bb58108',
          ledger_version: 0,
          network: 'mainnet',
          with_logs: false,
        }, {
          func: '0xb91d3fef0eeb4e685dc85e739c7d3e2968784945be4424e92e2f86e2418bf271::crank::peak',
          type_args: '',
          args: '0xd08a5107feb5f2df15c913702b0969ae4e22b3653a98c14fcd5e9e00cf8a039d\n2',
          ledger_version: 0,
          network: 'testnet',
          with_logs: false,
        }, {
          func: '0x6de517a18f003625e7fba9b9dc29b310f2e3026bbeb1997b3ada9de1e3cec8d6::opc::totalSupply',
          type_args: '',
          args: '',
          ledger_version: 0,
          network: 'mainnet',
          with_logs: false,
        },
      ];
      if (examples[n]) {
        this.message = examples[n];
      }
    },
  },
});
</script>
