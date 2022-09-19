# Firebase Admin SDK

This is a home-made client for Firebase's Admin SDK that seeks to provide a user friendly interface to interact with Firestore, Firebase Auth, and similar.

## Testing

Currently I've made the tests use a real in-the-cloud Firebase project to ensure that everything works as expected. However, this has the consequence that you need to set up access and be careful about tests affecting each other.

You can authenticate for an hour using your personal token for Google Cloud with this export:

```sh
export PROJECT_ID="<project id goes here>"
# Assumes you are logged in. Token will be valid for an hour.
export TOKEN=`gcloud auth print-access-token
```
