<script lang="ts">
	import { postApi } from '$lib/api';
	import { goto } from '$app/navigation';

	let email = '';
	let password = '';
	let errorMessage = '';

	async function handleRegister() {
		errorMessage = '';
		try {
			await postApi('register', { email, password });
			alert('Registration successful! Please log in.');
			goto('/login');
		} catch (error) {
			errorMessage = (error as Error).message || 'Registration failed.';
			console.error('Registration error:', error);
		}
	}
</script>

<div class="max-w-md mx-auto mt-10 p-6 bg-white rounded-lg shadow-md">
	<h2 class="text-2xl font-bold mb-6 text-center">Register</h2>
	<form on:submit|preventDefault={handleRegister}>
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
			Register
		</button>
	</form>
</div>