var app = angular.module('MainApp', ['ui.router']);

app.config(['$stateProvider', '$urlRouterProvider', function ($stateProvider, $urlRouterProvider) {

  $urlRouterProvider.otherwise('/home');


  $stateProvider.state('home', {
    url:'/home',
    resolve : {
      pageContent: function ($http) {
        return $http.get('../content_page.json').then(function (response) {
          return response;
        });
      }
    },
    views: {
      "main": {
        templateUrl: 'views/homepage.html',
        controller: 'homeController'
      }
    }
  });


  $stateProvider.state('api', {
    url:'/api',
    resolve : {
      apiStatus: function ($http) {
        return $http.get('../api.json').then(function (response) {
          return response;
        });
      }
    },
    views: {
      "main": {
        templateUrl: 'views/api.html',
        controller: 'apiController',
      }
    }   
  });

}]);


app.controller('apiController', ['$scope', 'apiStatus', function ($scope, apiStatus) {

  $scope.data = apiStatus.data;

}]);


app.controller('homeController', ['$scope', 'pageContent', function ($scope, pageContent) {

  var r = pageContent.data;

  $scope.title1 = r.title1;

  $scope.bigName = 'Dyam Marcano';
  console.info("Hello:", $scope.bigName);

}]);