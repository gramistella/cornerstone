<script lang="ts">
	import { postApi } from '$lib/api';
	import { token } from '$lib/auth';
	import { goto } from '$app/navigation';

	let email = '';
	let password = '';
	let errorMessage = '';

	async function handleLogin() {
		errorMessage = '';
		try {
			const response = await postApi('login', { email, password });
			if (response.token) {
				token.set(response.token);
				goto('/contacts');
			}
		} catch (error) {
			errorMessage = "Invalid email or password.";
			console.error('Login error:', error);
		}
	}
</script>

<div class="max-w-md mx-auto mt-10 p-6 bg-white rounded-lg shadow-md">
	<h2 class="text-2xl font-bold mb-6 text-center">Login</h2>
	<form on:submit|preventDefault={handleLogin}>
		{#if errorMessage}
			<div class="bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded relative mb-4" role="alert">
				<span class="block sm:inline">{errorMessage}</span>
			</div>
		{/if}
		<div class="mb-4">
			<label for="email" class="block text-gray-700">Email</label>
			<input type="email" id="email" bind:value={email} class="w-full px-3 py-2 border rounded" required />
		</div>
		<div class="mb-6">
			<label for="password" class="block text-gray-700">Password</label>
			<input type="password" id="password" bind:value={password} class="w-full px-3 py-2 border rounded" required />
		</div>
		<button type="submit" class="w-full bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded">
			Login
		</button>
	</form>
</div>