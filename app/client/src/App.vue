<template>
  <nav>
    <router-link to="/">Sentio Composer</router-link>
  </nav>
  <router-view />
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
  color: #2c3e50;
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
  color: #2c3e50;
  text-decoration: none;
}

nav a.router-link-exact-active {
  color: #42b983;
}
</style>
