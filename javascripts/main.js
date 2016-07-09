var app = angular.module('MainApp', ['ui.router']);

app.config(['$stateProvider', '$urlRouterProvider', function ($stateProvider, $urlRouterProvider) {

  $urlRouterProvider.otherwise('/home');

  $stateProvider.state('home', {
    url:'/home',
    views: {
    	"main": {
	    	templateUrl: 'views/homepage.html',
	    	controller: 'homeController'
    	}
    }
  });

  $stateProvider.state('api', {
    url:'/api',
    template: 'views/api.html',
    controller: 'apiController',
    resolve : {
      apiStatus: function ($http) {
        return $http.get('../api.json').then(function (response) {
          return response;
        });
      }
    }
  });

}]);

app.controller('apiController', ['$scope', 'apiStatus' function ($scope, apiStatus) {
	$scope.data = apiStatus.data;
}]);

app.controller('homeController', ['$scope', function ($scope) {
	
}]);