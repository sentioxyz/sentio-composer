<template>
  <nav>
    <router-link to="/">Sentio Composer</router-link>
    <div><a class="btn" href="https://github.com/sentioxyz/sentio-composer" rel="noopener" target="_blank" aria-label="Star sentio/sentio-composer"><svg viewBox="0 0 16 16" width="15" height="15" class="octicon octicon-mark-github" aria-hidden="true"><path fill-rule="evenodd" d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z"></path></svg>&nbsp;<span style="font-size:4mm">View source code on Github</span></a></div>
  </nav>
  <router-view />
  <footer>Brought to you by <a href="https://sentio.xyz">Sentio.xyz</a>. Powered by <a href="https://github.com/move-language/move">Move</a> and <a href="https://aptoslabs.com">Aptos</a>.</footer>
</template>

<style>
body {
  background: #f8fafc;
}

* {
  box-sizing: border-box;
}

#app {
  font-family: Avenir, Helvetica, Arial, sans-serif;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  color: #253E5D;
  width: 70rem;
  max-width: 100vw;
  margin: 0 auto;
}

nav {
  padding: 1rem 0;
  font-size: 1.25rem;
}

nav a {
  font-weight: bold;
  color: #253E5D;
  text-decoration: none;
}

nav a.router-link-exact-active {
  color: #20B2E4;
}

footer {
  margin: 2rem 0 1rem;
  text-align: center;
  font-size: 0.875rem;
}

footer a {
  color: inherit;
}
</style>
